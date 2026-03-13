"""Tests for the iRule test framework.

These tests verify:
1. The Tcl framework files exist and have correct structure
2. The topology bridge generates correct Tcl from SCF
3. The orchestrator event chains match MASTER_ORDER
4. Integration tests (when tclsh is available)
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import pytest

from core.bigip.model import (
    BigipConfig,
    BigipDataGroup,
    BigipPool,
    BigipPoolMember,
    BigipProfile,
    BigipRule,
    BigipVirtualServer,
    DataGroupType,
    ProfileType,
)
from core.irule_test.bridge import EventResult, IruleTestSession, RequestResult, _has_tkinter_tcl
from core.irule_test.codegen_event_data import _generate as generate_event_data
from core.irule_test.codegen_mock_stubs import _generate as generate_mock_stubs
from core.irule_test.codegen_registry_data import _generate as generate_registry_data
from core.irule_test.topology import TopologyFromSCF

# Path to the Tcl framework files
TCL_DIR = Path(__file__).parent.parent / "core" / "irule_test" / "tcl"

# Skip integration tests if no tclsh available
_tclsh = shutil.which("tclsh") or shutil.which("tclsh8.6") or shutil.which("tclsh8.5")
skip_no_tclsh = pytest.mark.skipif(_tclsh is None, reason="tclsh not available")


# Structural tests


class TestFrameworkFiles:
    """Verify all required Tcl framework files exist."""

    REQUIRED_FILES = [
        "compat84.tcl",
        "tmm_shim.tcl",
        "state_layers.tcl",
        "expr_ops.tcl",
        "command_mocks.tcl",
        "itest_core.tcl",
        "_event_data.tcl",
        "_registry_data.tcl",
        "orchestrator.tcl",
        "runner.tcl",
        "scf_loader.tcl",
        "example_test.tcl",
    ]

    @pytest.mark.parametrize("filename", REQUIRED_FILES)
    def test_file_exists(self, filename: str) -> None:
        path = TCL_DIR / filename
        assert path.exists(), f"Missing framework file: {filename}"

    @pytest.mark.parametrize("filename", REQUIRED_FILES)
    def test_file_not_empty(self, filename: str) -> None:
        path = TCL_DIR / filename
        assert path.stat().st_size > 0, f"Empty framework file: {filename}"


class TestTmmShimStructure:
    """Verify the TMM shim declares the right disabled commands."""

    def test_disabled_commands_include_exec(self) -> None:
        source = (TCL_DIR / "tmm_shim.tcl").read_text()
        assert "exec" in source

    def test_disabled_commands_include_socket(self) -> None:
        source = (TCL_DIR / "tmm_shim.tcl").read_text()
        assert "socket" in source

    def test_disabled_commands_include_open(self) -> None:
        source = (TCL_DIR / "tmm_shim.tcl").read_text()
        assert "open" in source

    def test_disabled_commands_include_file(self) -> None:
        source = (TCL_DIR / "tmm_shim.tcl").read_text()
        # file should be in the disabled list
        assert "file" in source

    def test_disabled_commands_include_source(self) -> None:
        source = (TCL_DIR / "tmm_shim.tcl").read_text()
        assert "source" in source

    def test_disabled_commands_include_package(self) -> None:
        source = (TCL_DIR / "tmm_shim.tcl").read_text()
        assert "package" in source

    def test_post84_commands_include_dict(self) -> None:
        source = (TCL_DIR / "tmm_shim.tcl").read_text()
        assert "dict" in source

    def test_reports_tcl84(self) -> None:
        source = (TCL_DIR / "tmm_shim.tcl").read_text()
        assert "8.4" in source


class TestExprOpsStructure:
    """Verify the expression operator module covers TMM operators."""

    TMM_OPERATORS = [
        "contains",
        "matches_regex",
        "starts_with",
        "ends_with",
        "equals",
        "matches_glob",
    ]

    @pytest.mark.parametrize("op", TMM_OPERATORS)
    def test_operator_implemented(self, op: str) -> None:
        source = (TCL_DIR / "expr_ops.tcl").read_text()
        assert f"_{op}" in source, f"Missing expression operator: {op}"


class TestOrchestratorMasterOrder:
    """Verify the generated event data has key events and chains."""

    def test_master_order_has_key_events(self) -> None:
        source = (TCL_DIR / "_event_data.tcl").read_text()
        key_events = [
            "RULE_INIT",
            "CLIENT_ACCEPTED",
            "CLIENTSSL_HANDSHAKE",
            "HTTP_REQUEST",
            "LB_SELECTED",
            "SERVER_CONNECTED",
            "HTTP_RESPONSE",
            "CLIENT_CLOSED",
        ]
        for event in key_events:
            assert event in source, f"Missing event in MASTER_ORDER: {event}"

    def test_flow_chains_defined(self) -> None:
        source = (TCL_DIR / "_event_data.tcl").read_text()
        chains = ["plain_tcp", "tcp_http", "tcp_clientssl_http", "udp_dns"]
        for chain in chains:
            assert chain in source, f"Missing flow chain: {chain}"


class TestFluentDSLStructure:
    """Verify the fluent assertion DSL procs are defined."""

    FLUENT_PROCS = [
        "assert_that",
        "_fluent_state",
        "_fluent_decision",
        "_fluent_log",
        "_fluent_event",
        "_fluent_http_header",
        "_fluent_response_header",
        "_fluent_var",
        "_fluent_compare",
        "_fluent_resolve_state",
    ]

    @pytest.mark.parametrize("proc_name", FLUENT_PROCS)
    def test_fluent_proc_exists(self, proc_name: str) -> None:
        source = (TCL_DIR / "orchestrator.tcl").read_text()
        assert f"proc {proc_name}" in source, f"Missing fluent DSL proc: {proc_name}"

    COMPARISON_VERBS = [
        "equals",
        "not_equals",
        "matches",
        "does_not_match",
        "contains",
        "does_not_contain",
        "starts_with",
        "ends_with",
    ]

    @pytest.mark.parametrize("verb", COMPARISON_VERBS)
    def test_comparison_verb_supported(self, verb: str) -> None:
        source = (TCL_DIR / "orchestrator.tcl").read_text()
        assert verb in source, f"Missing comparison verb: {verb}"

    STATE_PROPERTIES = [
        "pool_selected",
        "http_uri",
        "http_host",
        "http_method",
        "http_path",
        "http_status",
        "connection_state",
        "tls_sni",
        "client_addr",
        "server_addr",
    ]

    @pytest.mark.parametrize("prop", STATE_PROPERTIES)
    def test_state_property_mapped(self, prop: str) -> None:
        source = (TCL_DIR / "orchestrator.tcl").read_text()
        assert prop in source, f"Missing state property: {prop}"


class TestRequestResultAssertions:
    """Verify the Python assertion DSL on RequestResult."""

    def test_assert_pool_passes(self) -> None:
        r = RequestResult(pool_selected="web_pool")
        r.assert_pool("web_pool")  # should not raise

    def test_assert_pool_fails(self) -> None:
        r = RequestResult(pool_selected="web_pool")
        with pytest.raises(AssertionError, match="wrong_pool"):
            r.assert_pool("wrong_pool")

    def test_assert_decision_passes(self) -> None:
        r = RequestResult(decisions=[("lb", "pool_select", ["web_pool"])])
        r.assert_decision("lb", "pool_select", "web_pool")

    def test_assert_decision_without_value(self) -> None:
        r = RequestResult(decisions=[("lb", "pool_select", ["web_pool"])])
        r.assert_decision("lb", "pool_select")

    def test_assert_decision_fails(self) -> None:
        r = RequestResult(decisions=[])
        with pytest.raises(AssertionError, match="pool_select"):
            r.assert_decision("lb", "pool_select")

    def test_assert_no_decision_passes(self) -> None:
        r = RequestResult(decisions=[("lb", "pool_select", ["web_pool"])])
        r.assert_no_decision("connection", "reject")

    def test_assert_no_decision_fails(self) -> None:
        r = RequestResult(decisions=[("connection", "reject", [])])
        with pytest.raises(AssertionError, match="reject"):
            r.assert_no_decision("connection", "reject")

    def test_assert_event_fired(self) -> None:
        r = RequestResult(events_fired=[EventResult(event="HTTP_REQUEST", fired=True)])
        r.assert_event_fired("HTTP_REQUEST")

    def test_assert_event_not_fired(self) -> None:
        r = RequestResult(events_fired=[EventResult(event="HTTP_REQUEST", fired=False)])
        r.assert_event_not_fired("HTTP_REQUEST")

    def test_assert_connection_state(self) -> None:
        r = RequestResult(connection_state="closing")
        r.assert_connection_state("closing")

    def test_assert_connection_state_fails(self) -> None:
        r = RequestResult(connection_state="established")
        with pytest.raises(AssertionError, match="closing"):
            r.assert_connection_state("closing")


class TestCommandMocksStructure:
    """Verify key iRule commands are mocked."""

    REQUIRED_COMMANDS = [
        "IP::client_addr",
        "IP::local_addr",
        "TCP::collect",
        "HTTP::host",
        "HTTP::uri",
        "HTTP::header",
        "HTTP::respond",
        "HTTP::redirect",
        "SSL::cipher",
        "SSL::sni",
        "pool",
        "node",
        "snat",
        "class",
        "table",
        "persist",
        "event",
        "log",
        "reject",
        "drop",
    ]

    @pytest.mark.parametrize("cmd", REQUIRED_COMMANDS)
    def test_command_has_mock_proc(self, cmd: str) -> None:
        """Verify the mock proc exists for each required command."""
        source = (TCL_DIR / "command_mocks.tcl").read_text()
        # Derive expected mock proc name from command name
        if "::" in cmd:
            ns, sub = cmd.split("::", 1)
            proc_name = f"{ns.lower()}_{sub}"
        else:
            proc_name = f"cmd_{cmd}"
        assert f"proc {proc_name}" in source, f"Missing mock proc '{proc_name}' for command '{cmd}'"


# Topology bridge tests


class TestTopologyFromSCF:
    """Test the Python topology bridge."""

    @pytest.fixture()
    def sample_config(self) -> BigipConfig:
        """Build a sample BigipConfig for testing."""
        config = BigipConfig()
        config.virtual_servers["/Common/test_vs"] = BigipVirtualServer(
            name="test_vs",
            full_path="/Common/test_vs",
            destination="/Common/10.0.0.100:443",
            pool="/Common/web_pool",
            rules=("/Common/test_irule",),
            profiles=("/Common/http", "/Common/clientssl", "/Common/tcp"),
        )
        config.pools["/Common/web_pool"] = BigipPool(
            name="web_pool",
            full_path="/Common/web_pool",
            members=(
                BigipPoolMember(name="/Common/10.0.1.1:80"),
                BigipPoolMember(name="/Common/10.0.1.2:80"),
            ),
        )
        config.profiles["/Common/http"] = BigipProfile(
            name="http",
            full_path="/Common/http",
            profile_type=ProfileType.HTTP,
        )
        config.profiles["/Common/clientssl"] = BigipProfile(
            name="clientssl",
            full_path="/Common/clientssl",
            profile_type=ProfileType.CLIENT_SSL,
        )
        config.profiles["/Common/tcp"] = BigipProfile(
            name="tcp",
            full_path="/Common/tcp",
            profile_type=ProfileType.TCP,
        )
        config.rules["/Common/test_irule"] = BigipRule(
            name="test_irule",
            full_path="/Common/test_irule",
            source="when HTTP_REQUEST {\n    pool web_pool\n}",
        )
        config.data_groups["/Common/allowed"] = BigipDataGroup(
            name="allowed",
            full_path="/Common/allowed",
            kind=DataGroupType.INTERNAL,
            value_type="string",
            records=("host1", "host2"),
        )
        return config

    def test_generate_tcl_setup_profiles(self, sample_config: BigipConfig) -> None:
        topo = TopologyFromSCF(sample_config)
        tcl = topo.generate_tcl_setup("/Common/test_vs")
        assert "TCP" in tcl
        assert "HTTP" in tcl
        assert "CLIENTSSL" in tcl

    def test_generate_tcl_setup_pools(self, sample_config: BigipConfig) -> None:
        topo = TopologyFromSCF(sample_config)
        tcl = topo.generate_tcl_setup("/Common/test_vs")
        assert "web_pool" in tcl
        assert "10.0.1.1:80" in tcl

    def test_generate_tcl_setup_vip(self, sample_config: BigipConfig) -> None:
        topo = TopologyFromSCF(sample_config)
        tcl = topo.generate_tcl_setup("/Common/test_vs")
        assert "10.0.0.100" in tcl
        assert "443" in tcl

    def test_generate_tcl_setup_irule(self, sample_config: BigipConfig) -> None:
        topo = TopologyFromSCF(sample_config)
        tcl = topo.generate_tcl_setup("/Common/test_vs")
        assert "load_irule" in tcl
        assert "HTTP_REQUEST" in tcl

    def test_generate_tcl_setup_datagroups(self, sample_config: BigipConfig) -> None:
        topo = TopologyFromSCF(sample_config)
        tcl = topo.generate_tcl_setup("/Common/test_vs")
        assert "allowed" in tcl

    def test_virtual_servers_list(self, sample_config: BigipConfig) -> None:
        topo = TopologyFromSCF(sample_config)
        assert "/Common/test_vs" in topo.virtual_servers()

    def test_generate_full_test_script(self, sample_config: BigipConfig) -> None:
        topo = TopologyFromSCF(sample_config)
        script = topo.generate_full_test_script("/Common/test_vs")
        assert "#!/usr/bin/env tclsh" in script
        assert "::orch::init" in script
        assert "::orch::summary" in script

    def test_unknown_vs_raises(self, sample_config: BigipConfig) -> None:
        topo = TopologyFromSCF(sample_config)
        with pytest.raises(KeyError, match="not_real"):
            topo.generate_tcl_setup("not_real")


class TestTopologyFromSCFParsing:
    """Test parsing real SCF content."""

    SAMPLE_SCF = """\
ltm pool /Common/web_pool {
    members {
        /Common/10.0.1.1:80 {
            address 10.0.1.1
        }
        /Common/10.0.1.2:80 {
            address 10.0.1.2
        }
    }
    monitor /Common/http
}

ltm profile http /Common/http { }
ltm profile client-ssl /Common/my_clientssl { }
ltm profile tcp /Common/tcp { }

ltm rule /Common/my_irule {
    when HTTP_REQUEST {
        if { [HTTP::host] eq "api.example.com" } {
            pool web_pool
        }
    }
}

ltm virtual /Common/my_vs {
    destination /Common/10.0.0.1:443
    pool /Common/web_pool
    profiles {
        /Common/http { }
        /Common/my_clientssl {
            context clientside
        }
        /Common/tcp { }
    }
    rules {
        /Common/my_irule
    }
}
"""

    def test_parse_scf_string(self) -> None:
        topo = TopologyFromSCF.from_string(self.SAMPLE_SCF)
        assert "/Common/my_vs" in topo.virtual_servers()

    def test_parse_scf_generates_tcl(self) -> None:
        topo = TopologyFromSCF.from_string(self.SAMPLE_SCF)
        tcl = topo.generate_tcl_setup("/Common/my_vs")
        assert "web_pool" in tcl
        assert "10.0.0.1" in tcl


# Integration tests (require tclsh)


def _tclsh_path() -> str:
    """Return tclsh path, asserting it exists (guarded by skip_no_tclsh)."""
    assert _tclsh is not None
    return _tclsh


@skip_no_tclsh
class TestTclIntegration:
    """Run actual Tcl tests when tclsh is available."""

    def test_example_test_runs(self) -> None:
        """Run the example test script and verify it passes."""
        result = subprocess.run(
            [_tclsh_path(), str(TCL_DIR / "example_test.tcl")],
            capture_output=True,
            text=True,
            timeout=30,
        )
        # The example should complete without errors
        assert result.returncode == 0, f"stderr: {result.stderr}\nstdout: {result.stdout}"
        assert "FAIL" not in result.stdout or "0 FAILED" in result.stdout

    def test_compat84_loads(self) -> None:
        """Verify the compat84 shim loads without errors."""
        script = f'source "{TCL_DIR / "compat84.tcl"}"\nputs "ok"'
        result = subprocess.run(
            [_tclsh_path()],
            input=script,
            capture_output=True,
            text=True,
            timeout=10,
        )
        assert result.returncode == 0, f"stderr: {result.stderr}"
        assert "ok" in result.stdout

    def test_state_layers_load(self) -> None:
        """Verify state layers load and basic operations work."""
        script = f"""
source "{TCL_DIR / "compat84.tcl"}"
source "{TCL_DIR / "state_layers.tcl"}"
::state::connection::configure -client_addr 1.2.3.4
puts $::state::connection::client_addr
"""
        result = subprocess.run(
            [_tclsh_path()],
            input=script,
            capture_output=True,
            text=True,
            timeout=10,
        )
        assert result.returncode == 0, f"stderr: {result.stderr}"
        assert "1.2.3.4" in result.stdout

    def test_tmm_shim_blocks_exec(self) -> None:
        """Verify exec is blocked after TMM shim init."""
        script = f"""
source "{TCL_DIR / "compat84.tcl"}"
source "{TCL_DIR / "state_layers.tcl"}"
source "{TCL_DIR / "tmm_shim.tcl"}"
::tmm::init
if {{[catch {{exec echo hello}} err]}} {{
    puts "blocked: $err"
}} else {{
    puts "NOT BLOCKED"
}}
"""
        result = subprocess.run(
            [_tclsh_path()],
            input=script,
            capture_output=True,
            text=True,
            timeout=10,
        )
        assert result.returncode == 0, f"stderr: {result.stderr}"
        assert "blocked" in result.stdout
        assert "NOT BLOCKED" not in result.stdout

    def test_tmm_shim_reports_84(self) -> None:
        """Verify info tclversion reports 8.4 after shim init."""
        script = f"""
source "{TCL_DIR / "compat84.tcl"}"
source "{TCL_DIR / "state_layers.tcl"}"
source "{TCL_DIR / "tmm_shim.tcl"}"
::tmm::init
puts [info tclversion]
"""
        result = subprocess.run(
            [_tclsh_path()],
            input=script,
            capture_output=True,
            text=True,
            timeout=10,
        )
        assert result.returncode == 0, f"stderr: {result.stderr}"
        assert "8.4" in result.stdout

    def test_expr_ops_contains(self) -> None:
        """Test the 'contains' expression operator."""
        script = f"""
source "{TCL_DIR / "compat84.tcl"}"
source "{TCL_DIR / "state_layers.tcl"}"
source "{TCL_DIR / "tmm_shim.tcl"}"
source "{TCL_DIR / "expr_ops.tcl"}"
::tmm::init
::tmm::expr_ops::install
if {{ "hello world" contains "world" }} {{
    puts "CONTAINS_OK"
}} else {{
    puts "CONTAINS_FAIL"
}}
"""
        result = subprocess.run(
            [_tclsh_path()],
            input=script,
            capture_output=True,
            text=True,
            timeout=10,
        )
        assert result.returncode == 0, f"stderr: {result.stderr}"
        assert "CONTAINS_OK" in result.stdout


# Bridge backend tests


class TestBridgeBackendSelection:
    """Test backend auto-selection logic."""

    def test_has_tkinter_tcl_returns_bool(self) -> None:
        result = _has_tkinter_tcl()
        assert isinstance(result, bool)

    def test_auto_selects_a_backend(self) -> None:
        """Auto should pick inprocess or subprocess (not raise)."""
        try:
            session = IruleTestSession(backend="auto")
            assert session.backend_type in ("inprocess", "subprocess")
        except RuntimeError:
            pytest.skip("No Tcl interpreter available")

    def test_unknown_backend_raises(self) -> None:
        with pytest.raises(ValueError, match="Unknown backend"):
            IruleTestSession(backend="nonexistent")

    def test_itest_core_file_exists(self) -> None:
        """Verify itest_core.tcl exists (extracted from runner.tcl)."""
        assert (TCL_DIR / "itest_core.tcl").exists()

    def test_itest_core_has_load_irule(self) -> None:
        source = (TCL_DIR / "itest_core.tcl").read_text()
        assert "proc load_irule" in source

    def test_itest_core_has_fire_event(self) -> None:
        source = (TCL_DIR / "itest_core.tcl").read_text()
        assert "proc fire_event" in source

    def test_runner_sources_itest_core(self) -> None:
        source = (TCL_DIR / "runner.tcl").read_text()
        assert "itest_core.tcl" in source

    def test_example_sources_itest_core(self) -> None:
        source = (TCL_DIR / "example_test.tcl").read_text()
        assert "itest_core.tcl" in source


class TestEventDataCodegen:
    """Verify the generated _event_data.tcl is up to date."""

    def test_generated_file_is_current(self) -> None:
        """Fail if _event_data.tcl differs from what codegen would produce."""
        expected = generate_event_data()
        actual = (TCL_DIR / "_event_data.tcl").read_text()
        assert actual == expected, (
            "_event_data.tcl is stale. Regenerate with: "
            "python -m core.irule_test.codegen_event_data"
        )

    def test_generated_file_has_master_order(self) -> None:
        source = (TCL_DIR / "_event_data.tcl").read_text()
        assert "variable MASTER_ORDER" in source

    def test_generated_file_has_flow_chains(self) -> None:
        source = (TCL_DIR / "_event_data.tcl").read_text()
        assert "FLOW_CHAINS" in source
        assert "plain_tcp" in source
        assert "tcp_http" in source
        assert "tcp_clientssl_http" in source
        assert "udp_dns" in source
        assert "tcp_dns" in source

    def test_generated_file_has_all_python_chains(self) -> None:
        """All 7 Python flow chains should appear in the generated Tcl."""
        from core.commands.registry.namespace_data import FLOW_CHAINS

        source = (TCL_DIR / "_event_data.tcl").read_text()
        for chain_id in FLOW_CHAINS:
            assert chain_id in source, f"Missing flow chain: {chain_id}"

    def test_orchestrator_sources_event_data(self) -> None:
        source = (TCL_DIR / "orchestrator.tcl").read_text()
        assert "_event_data.tcl" in source

    def test_orchestrator_has_no_inline_master_order(self) -> None:
        """MASTER_ORDER definition should come from _event_data.tcl, not inline."""
        source = (TCL_DIR / "orchestrator.tcl").read_text()
        # orchestrator.tcl may *reference* MASTER_ORDER (via `variable MASTER_ORDER`)
        # but must not *define* it with a value (i.e., no `variable MASTER_ORDER {`)
        assert "variable MASTER_ORDER {" not in source


class TestRegistryDataCodegen:
    """Verify the generated _registry_data.tcl is up to date."""

    def test_generated_file_is_current(self) -> None:
        """Fail if _registry_data.tcl differs from what codegen would produce."""
        expected = generate_registry_data()
        actual = (TCL_DIR / "_registry_data.tcl").read_text()
        assert actual == expected, (
            "_registry_data.tcl is stale. Regenerate with: "
            "python -m core.irule_test.codegen_registry_data"
        )

    def test_has_disabled_commands(self) -> None:
        source = (TCL_DIR / "_registry_data.tcl").read_text()
        assert "_gen_disabled_commands" in source
        # Key commands that TMM definitely removes
        assert "exec" in source
        assert "socket" in source
        assert "glob" in source

    def test_has_post84_commands(self) -> None:
        source = (TCL_DIR / "_registry_data.tcl").read_text()
        assert "_gen_post84_commands" in source

    def test_has_tmm_operators(self) -> None:
        source = (TCL_DIR / "_registry_data.tcl").read_text()
        assert "_gen_operators" in source
        assert "contains" in source
        assert "starts_with" in source
        assert "ends_with" in source
        assert "matches_regex" in source

    def test_has_irule_commands(self) -> None:
        source = (TCL_DIR / "_registry_data.tcl").read_text()
        assert "_gen_namespaced_commands" in source
        assert "_gen_toplevel_commands" in source
        assert "HTTP::header" in source
        assert "IP::client_addr" in source

    def test_tmm_shim_uses_generated_data(self) -> None:
        source = (TCL_DIR / "tmm_shim.tcl").read_text()
        assert "_registry_data.tcl" in source
        assert "_gen_disabled_commands" in source
        assert "_gen_post84_commands" in source
        # Should NOT have inline command lists
        assert "variable disabled_commands {" not in source
        assert "variable post84_commands {" not in source

    def test_expr_ops_uses_generated_data(self) -> None:
        source = (TCL_DIR / "expr_ops.tcl").read_text()
        assert "_gen_operators" in source
        # Should NOT have inline operator list
        assert "variable _tmm_operators {" not in source

    def test_command_mocks_uses_generated_data(self) -> None:
        source = (TCL_DIR / "command_mocks.tcl").read_text()
        assert "_gen_namespaced_commands" in source
        assert "_gen_toplevel_commands" in source

    def test_operators_match_registry(self) -> None:
        from core.commands.registry.operators import IRULES_OPERATOR_HOVER

        source = (TCL_DIR / "_registry_data.tcl").read_text()
        for op in IRULES_OPERATOR_HOVER:
            if op not in ("and", "or", "not"):
                assert op in source, f"Missing operator: {op}"


skip_no_tkinter = pytest.mark.skipif(not _has_tkinter_tcl(), reason="tkinter.Tcl() not available")


@skip_no_tkinter
class TestInProcessBackend:
    """Integration tests for the in-process tkinter.Tcl() backend."""

    def test_session_starts_inprocess(self) -> None:
        import asyncio

        async def _run() -> None:
            async with IruleTestSession(backend="inprocess") as session:
                assert session.backend_type == "inprocess"

        asyncio.new_event_loop().run_until_complete(_run())

    def test_load_irule_returns_events(self) -> None:
        import asyncio

        async def _run() -> list[str]:
            async with IruleTestSession(backend="inprocess") as session:
                return await session.load_irule_async("when HTTP_REQUEST { pool web_pool }")

        events = asyncio.new_event_loop().run_until_complete(_run())
        assert "HTTP_REQUEST" in events

    def test_fire_event_with_handler(self) -> None:
        import asyncio

        async def _run() -> None:
            async with IruleTestSession(backend="inprocess", profiles=["TCP", "HTTP"]) as session:
                await session.add_pool("web_pool", ["10.0.1.1:80"])
                await session.load_irule_async("when HTTP_REQUEST { pool web_pool }")
                result = await session.fire_event("HTTP_REQUEST")
                assert result.fired is True

        asyncio.new_event_loop().run_until_complete(_run())

    def test_fire_event_no_handler(self) -> None:
        import asyncio

        async def _run() -> None:
            async with IruleTestSession(backend="inprocess") as session:
                await session.load_irule_async("when HTTP_REQUEST { pool web_pool }")
                result = await session.fire_event("CLIENT_ACCEPTED")
                assert result.fired is False

        asyncio.new_event_loop().run_until_complete(_run())

    def test_pool_selection(self) -> None:
        import asyncio

        async def _run() -> str:
            async with IruleTestSession(backend="inprocess", profiles=["TCP", "HTTP"]) as session:
                await session.add_pool("web_pool", ["10.0.1.1:80"])
                await session.load_irule_async("when HTTP_REQUEST { pool web_pool }")
                await session.fire_event("HTTP_REQUEST")
                state = await session.get_state("lb")
                return state.get("pool", "")

        pool = asyncio.new_event_loop().run_until_complete(_run())
        assert pool == "web_pool"

    def test_state_read_write(self) -> None:
        import asyncio

        async def _run() -> dict[str, str]:
            async with IruleTestSession(backend="inprocess") as session:
                await session.set_state("connection", {"client_addr": "1.2.3.4"})
                return await session.get_state("connection")

        state = asyncio.new_event_loop().run_until_complete(_run())
        assert state["client_addr"] == "1.2.3.4"

    def test_eval_tcl(self) -> None:
        import asyncio

        async def _run() -> str:
            async with IruleTestSession(backend="inprocess") as session:
                resp = await session._send({"cmd": "eval", "script": "expr {2 + 3}"})
                return resp.get("result", "")

        result = asyncio.new_event_loop().run_until_complete(_run())
        assert result == "5"

    def test_reset(self) -> None:
        import asyncio

        async def _run() -> str:
            async with IruleTestSession(backend="inprocess", profiles=["TCP", "HTTP"]) as session:
                await session.add_pool("web_pool", ["10.0.1.1:80"])
                await session.load_irule_async("when HTTP_REQUEST { pool web_pool }")
                await session.fire_event("HTTP_REQUEST")
                await session.reset("all")
                state = await session.get_state("lb")
                return state.get("pool", "")

        pool = asyncio.new_event_loop().run_until_complete(_run())
        assert pool == ""


@skip_no_tkinter
class TestFluentAssertionDSL:
    """Integration tests for the fluent assertion DSL (::orch::assert_that)."""

    IRULE = 'when HTTP_REQUEST { pool web_pool ; HTTP::header insert "X-Test" "yes" }'

    @staticmethod
    def _eval(script: str) -> str:
        """Run a Tcl script in a fresh in-process session and return result."""
        import asyncio

        async def _run() -> str:
            async with IruleTestSession(backend="inprocess", profiles=["TCP", "HTTP"]) as session:
                await session.add_pool("web_pool", ["10.0.1.1:80"])
                resp = await session._send({"cmd": "eval", "script": script})
                return resp.get("result", "")

        return asyncio.new_event_loop().run_until_complete(_run())

    def test_assert_that_pool_selected_equals(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that pool_selected equals web_pool
        """)
        assert result == "1"

    def test_assert_that_pool_selected_fails(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that pool_selected equals wrong_pool
        """)
        assert result == "0"

    def test_assert_that_http_host_equals(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host api.example.com
            ::orch::assert_that http_host equals api.example.com
        """)
        assert result == "1"

    def test_assert_that_http_uri_contains(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com -uri /v1/health
            ::orch::assert_that http_uri contains health
        """)
        assert result == "1"

    def test_assert_that_http_uri_starts_with(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com -uri /v1/health
            ::orch::assert_that http_uri starts_with /v1
        """)
        assert result == "1"

    def test_assert_that_decision_was_called(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that decision lb pool_select was_called
        """)
        assert result == "1"

    def test_assert_that_decision_was_called_with(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that decision lb pool_select was_called_with web_pool
        """)
        assert result == "1"

    def test_assert_that_decision_was_not_called(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that decision connection reject was_not_called
        """)
        assert result == "1"

    def test_assert_that_log_matches(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { log local0. "hello world" }}
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that log matches "*hello*"
        """)
        assert result == "1"

    def test_assert_that_log_does_not_match(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that log does_not_match "*error*"
        """)
        assert result == "1"

    def test_assert_that_event_is_registered(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::assert_that event HTTP_REQUEST is_registered
        """)
        assert result == "1"

    def test_assert_that_event_is_not_registered(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::assert_that event CLIENT_ACCEPTED is_not_registered
        """)
        assert result == "1"

    def test_assert_that_http_header_equals(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com -headers {X-Custom myval}
            ::orch::assert_that http_header X-Custom equals myval
        """)
        assert result == "1"

    def test_assert_that_connection_state(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { reject }}
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that connection_state equals closing
        """)
        assert result == "1"

    def test_assert_that_not_equals(self) -> None:
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that pool_selected not_equals wrong_pool
        """)
        assert result == "1"

    def test_summary_counts_fluent_assertions(self) -> None:
        """Fluent assertions share the counter with classic assertions."""
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that pool_selected equals web_pool
            ::orch::assert_pool_selected web_pool
            set ::orch::_assert_count
        """)
        assert result == "2"


@skip_no_tkinter
class TestKeepAlive:
    """Integration tests for keep-alive / multi-request orchestration."""

    @staticmethod
    def _eval(script: str) -> str:
        import asyncio

        async def _run() -> str:
            async with IruleTestSession(backend="inprocess", profiles=["TCP", "HTTP"]) as session:
                await session.add_pool("api_pool", ["10.0.1.1:80"])
                await session.add_pool("web_pool", ["10.0.2.1:80"])
                resp = await session._send({"cmd": "eval", "script": script})
                return resp.get("result", "")

        return asyncio.new_event_loop().run_until_complete(_run())

    def test_tcl_keep_alive_pool_changes(self) -> None:
        """Two requests on same connection, different pools."""
        result = self._eval("""
            ::orch::load_irule {
                when HTTP_REQUEST {
                    if {[HTTP::host] eq "api.example.com"} {
                        pool api_pool
                    } else {
                        pool web_pool
                    }
                }
            }
            ::orch::run_http_request -host api.example.com
            set first_pool $::state::lb::pool

            ::orch::run_next_request -host web.example.com
            set second_pool $::state::lb::pool

            list $first_pool $second_pool
        """)
        assert result == "api_pool web_pool"

    def test_tcl_connection_state_persists(self) -> None:
        """Connection state survives across keep-alive requests."""
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool api_pool }}
            ::orch::run_http_request -host test.example.com
            set addr1 $::state::connection::client_addr

            ::orch::run_next_request -host test.example.com
            set addr2 $::state::connection::client_addr

            expr {$addr1 eq $addr2}
        """)
        assert result == "1"

    def test_tcl_decisions_reset_between_requests(self) -> None:
        """Per-request decisions reset between keep-alive requests."""
        result = self._eval("""
            ::orch::load_irule {when HTTP_REQUEST { pool api_pool }}
            ::orch::run_http_request -host test.example.com

            ::orch::run_next_request -host test.example.com
            set decisions [::itest::get_decisions lb]
            # Should only have decisions from the second request
            llength $decisions
        """)
        # Should have exactly 1 decision (pool_select from second request)
        assert result == "1"

    def test_tcl_close_connection(self) -> None:
        """close_connection fires CLIENT_CLOSED and resets state."""
        result = self._eval("""
            ::orch::load_irule {
                when HTTP_REQUEST { pool api_pool }
                when CLIENT_CLOSED { log local0. "connection closed" }
            }
            ::orch::run_http_request -host test.example.com
            ::orch::close_connection
            set ::orch::_connection_active
        """)
        assert result == "0"

    def test_python_run_http_request_via_orchestrator(self) -> None:
        """Python run_http_request delegates to Tcl orchestrator."""
        import asyncio

        async def _run() -> RequestResult:
            async with IruleTestSession(backend="inprocess", profiles=["TCP", "HTTP"]) as session:
                await session.add_pool("web_pool", ["10.0.1.1:80"])
                await session.load_irule_async("when HTTP_REQUEST { pool web_pool }")
                return await session.run_http_request(host="test.example.com")

        result = asyncio.new_event_loop().run_until_complete(_run())
        result.assert_pool("web_pool")

    def test_python_keep_alive_sequence(self) -> None:
        """Python bridge supports keep-alive via run_next_request."""
        import asyncio

        async def _run() -> tuple[RequestResult, RequestResult]:
            async with IruleTestSession(backend="inprocess", profiles=["TCP", "HTTP"]) as session:
                await session.add_pool("api_pool", ["10.0.1.1:80"])
                await session.add_pool("web_pool", ["10.0.2.1:80"])
                await session.load_irule_async("""
                    when HTTP_REQUEST {
                        if {[HTTP::host] eq "api.example.com"} {
                            pool api_pool
                        } else {
                            pool web_pool
                        }
                    }
                """)
                r1 = await session.run_http_request(host="api.example.com")
                r2 = await session.run_next_request(host="web.example.com")
                return r1, r2

        r1, r2 = asyncio.new_event_loop().run_until_complete(_run())
        r1.assert_pool("api_pool")
        r2.assert_pool("web_pool")

    def test_python_close_connection(self) -> None:
        """Python bridge supports close_connection."""
        import asyncio

        async def _run() -> None:
            async with IruleTestSession(backend="inprocess", profiles=["TCP", "HTTP"]) as session:
                await session.add_pool("web_pool", ["10.0.1.1:80"])
                await session.load_irule_async("when HTTP_REQUEST { pool web_pool }")
                await session.run_http_request(host="test.example.com")
                await session.close_connection()
                # After close, a new run_http_request starts fresh
                result = await session.run_http_request(host="test.example.com")
                result.assert_pool("web_pool")

        asyncio.new_event_loop().run_until_complete(_run())


@skip_no_tkinter
class TestStaticVariableLifecycle:
    """Integration tests for static:: variable persistence."""

    @staticmethod
    def _eval(script: str) -> str:
        import asyncio

        async def _run() -> str:
            async with IruleTestSession(backend="inprocess", profiles=["TCP", "HTTP"]) as session:
                await session.add_pool("web_pool", ["10.0.1.1:80"])
                resp = await session._send({"cmd": "eval", "script": script})
                return resp.get("result", "")

        return asyncio.new_event_loop().run_until_complete(_run())

    def test_static_set_in_rule_init(self) -> None:
        """static:: variables set in RULE_INIT persist to HTTP_REQUEST."""
        result = self._eval("""
            ::orch::load_irule {
                when RULE_INIT { set static::counter 0 }
                when HTTP_REQUEST {
                    set static::counter [expr {$static::counter + 1}]
                    pool web_pool
                }
            }
            ::orch::run_http_request -host test.example.com
            set ::static::counter
        """)
        assert result == "1"

    def test_static_persists_across_requests(self) -> None:
        """static:: variables persist across keep-alive requests."""
        result = self._eval("""
            ::orch::load_irule {
                when RULE_INIT { set static::counter 0 }
                when HTTP_REQUEST {
                    set static::counter [expr {$static::counter + 1}]
                    pool web_pool
                }
            }
            ::orch::run_http_request -host test.example.com
            ::orch::run_next_request -host test.example.com
            set ::static::counter
        """)
        assert result == "2"

    def test_static_persists_across_connections(self) -> None:
        """static:: variables persist across connection close/reopen."""
        result = self._eval("""
            ::orch::load_irule {
                when RULE_INIT { set static::counter 0 }
                when HTTP_REQUEST {
                    set static::counter [expr {$static::counter + 1}]
                    pool web_pool
                }
            }
            ::orch::run_http_request -host test.example.com
            ::orch::close_connection
            ::orch::run_http_request -host test.example.com
            set ::static::counter
        """)
        # RULE_INIT only fires once, counter incremented in both HTTP_REQUESTs
        assert result == "2"

    def test_configure_static_helper(self) -> None:
        """::orch::configure_static pre-seeds static variables."""
        result = self._eval("""
            ::orch::configure_static debug 1
            ::orch::load_irule {
                when HTTP_REQUEST {
                    if {$static::debug} {
                        log local0. "debug mode"
                    }
                    pool web_pool
                }
            }
            ::orch::run_http_request -host test.example.com
            ::orch::assert_that log matches "*debug mode*"
        """)
        assert result == "1"

    def test_fluent_assert_static_var(self) -> None:
        """Fluent DSL can assert on static:: variables."""
        result = self._eval("""
            ::orch::configure_static myvar hello
            ::orch::load_irule {when HTTP_REQUEST { pool web_pool }}
            ::orch::assert_that var static::myvar equals hello
        """)
        assert result == "1"

    def test_reset_all_clears_statics(self) -> None:
        """::orch::reset clears static:: variables."""
        result = self._eval("""
            ::orch::configure_static myvar hello
            ::orch::reset
            if {[::info exists ::static::myvar]} { return exists } else { return cleared }
        """)
        assert result == "cleared"


# Phase 6 tests: Mock generator + command coverage


class TestMockStubsCodegen:
    """Verify the mock stubs codegen produces valid, up-to-date output."""

    def test_stubs_file_exists(self) -> None:
        assert (TCL_DIR / "_mock_stubs.tcl").exists()

    def test_stubs_not_stale(self) -> None:
        """Generated stubs file matches in-memory generation."""
        on_disk = (TCL_DIR / "_mock_stubs.tcl").read_text()
        fresh = generate_mock_stubs()
        assert on_disk == fresh, (
            "_mock_stubs.tcl is stale -- regenerate with: "
            "python -m core.irule_test.codegen_mock_stubs"
        )

    def test_stubs_has_namespace_eval(self) -> None:
        content = (TCL_DIR / "_mock_stubs.tcl").read_text()
        assert "namespace eval ::itest::cmd" in content

    def test_stubs_cover_unmocked_commands(self) -> None:
        """Every iRule command should have either a hand-written mock or a stub."""
        import re

        mocks_content = (TCL_DIR / "command_mocks.tcl").read_text()
        stubs_content = (TCL_DIR / "_mock_stubs.tcl").read_text()
        combined = mocks_content + stubs_content
        all_procs = set(re.findall(r"proc\s+([\w_]+)\s", combined))

        from core.commands.registry.command_registry import REGISTRY
        from core.irule_test.codegen_mock_stubs import _mock_proc_name

        irule_cmds = REGISTRY.command_names(dialect="f5-irules")
        missing = []
        for cmd in irule_cmds:
            proc_name = _mock_proc_name(cmd)
            if proc_name not in all_procs:
                missing.append(cmd)

        assert not missing, f"Commands without mock or stub: {missing}"


class TestTier1DiagramActionMocks:
    """Verify all 35 diagram-action commands have hand-written mocks."""

    DIAGRAM_ACTION_COMMANDS = [
        "DNS::answer",
        "DNS::header",
        "DNS::return",
        "HTTP::close",
        "HTTP::collect",
        "HTTP::cookie",
        "HTTP::header",
        "HTTP::host",
        "HTTP::path",
        "HTTP::redirect",
        "HTTP::release",
        "HTTP::respond",
        "HTTP::retry",
        "HTTP::uri",
        "SSL::disable",
        "SSL::enable",
        "SSL::respond",
        "TCP::close",
        "TCP::collect",
        "TCP::release",
        "TCP::respond",
        "after",
        "class",
        "discard",
        "drop",
        "event",
        "log",
        "node",
        "persist",
        "pool",
        "reject",
        "snat",
        "snatpool",
        "table",
        "virtual",
    ]

    @pytest.mark.parametrize("cmd", DIAGRAM_ACTION_COMMANDS)
    def test_diagram_action_has_handwritten_mock(self, cmd: str) -> None:
        """Each diagram-action command has a handwritten mock, not just a stub."""
        import re

        content = (TCL_DIR / "command_mocks.tcl").read_text()
        all_procs = set(re.findall(r"proc\s+([\w_]+)\s", content))

        if "::" in cmd:
            parts = cmd.split("::")
            proc_name = f"{parts[0].lower()}_{parts[-1]}"
        else:
            proc_name = f"cmd_{cmd}"

        assert proc_name in all_procs, (
            f"Diagram-action command {cmd} needs a hand-written mock "
            f"(expected proc {proc_name} in command_mocks.tcl)"
        )


class TestDNSState:
    """Verify DNS state layer exists in state_layers.tcl."""

    def test_dns_namespace_exists(self) -> None:
        content = (TCL_DIR / "state_layers.tcl").read_text()
        assert "namespace eval dns {" in content

    def test_dns_has_reset(self) -> None:
        content = (TCL_DIR / "state_layers.tcl").read_text()
        assert "variable qname" in content
        assert "variable rcode" in content


@pytest.mark.skipif(not _has_tkinter_tcl(), reason="tkinter.Tcl() not available")
class TestMockStubsIntegration:
    """Integration tests: verify stubs work in the Tcl interpreter."""

    session: IruleTestSession

    @pytest.fixture(autouse=True)
    def _setup_session(self) -> None:
        import asyncio

        async def _setup() -> None:
            self.session = IruleTestSession(backend="inprocess")
            await self.session.start()

        asyncio.new_event_loop().run_until_complete(_setup())

    def _eval(self, script: str) -> str:
        import asyncio

        async def _run() -> str:
            resp = await self.session._send({"command": "eval", "script": script})
            return resp.get("result", "")

        return asyncio.new_event_loop().run_until_complete(_run())

    def test_stub_command_does_not_crash(self) -> None:
        """A stub-only command (e.g. DIAMETER::avp) doesn't error."""
        result = self._eval("""
            ::orch::load_irule {
                when CLIENT_ACCEPTED { DIAMETER::avp insert 263 "session-1" }
            }
            ::orch::configure -profiles {TCP}
            ::orch::run_http_request
            return ok
        """)
        assert result == "ok"

    def test_stub_command_logs_decision(self) -> None:
        """Stub commands log to the decision log."""
        result = self._eval("""
            ::orch::load_irule {
                when CLIENT_ACCEPTED { MQTT::publish "topic" "payload" }
            }
            ::orch::configure -profiles {TCP}
            ::orch::run_http_request
            set decisions [::itest::get_decisions mqtt]
            llength $decisions
        """)
        assert int(result) >= 1

    def test_dns_answer_mock(self) -> None:
        """DNS::answer insert works and records count."""
        result = self._eval("""
            DNS::answer insert -type A -name "test.com" -rdata "1.2.3.4"
            DNS::answer count
        """)
        assert result == "1"

    def test_dns_header_mock(self) -> None:
        """DNS::header reads/writes DNS state."""
        result = self._eval("""
            set ::state::dns::qname "example.com"
            DNS::header qname
        """)
        assert result == "example.com"

    def test_dns_return_mock(self) -> None:
        """DNS::return sets response_sent flag."""
        result = self._eval("""
            DNS::return
            set ::state::dns::response_sent
        """)
        assert result == "1"

    def test_http_close_mock(self) -> None:
        """HTTP::close marks connection as closing."""
        result = self._eval("""
            HTTP::close
            set ::state::connection::state
        """)
        assert result == "closing"

    def test_ssl_disable_mock(self) -> None:
        """SSL::disable logs decision."""
        result = self._eval("""
            SSL::disable serverside
            set d [::itest::get_decisions ssl]
            lindex [lindex $d 0] 1
        """)
        assert result == "disable"

    def test_virtual_getter(self) -> None:
        """virtual with no args returns VIP address."""
        result = self._eval("""
            virtual
        """)
        assert result == "192.168.1.100"

    def test_virtual_redirect(self) -> None:
        """virtual <name> logs LB decision."""
        result = self._eval("""
            virtual other_vs
            set d [::itest::get_decisions lb]
            lindex [lindex $d 0] 2
        """)
        assert result == "other_vs"

    def test_all_commands_registered(self) -> None:
        """After init, all iRule commands should be registered."""
        result = self._eval("""
            # Count registered commands
            set count 0
            foreach {k v} [array get ::itest::_command_map] {
                incr count
            }
            return $count
        """)
        # Should have a large number of registered commands
        assert int(result) > 100


# Phase 7 tests: AI test generation


class TestGenerateTest:
    """Verify the test generation skill produces valid output."""

    SAMPLE_IRULE = """\
when HTTP_REQUEST {
    set host [HTTP::host]
    if { $host eq "api.example.com" } {
        pool api_pool
    } elseif { [class match $host equals allowed_hosts] } {
        pool web_pool
    } else {
        reject
    }
    log local0. "Routed $host"
}
"""

    def test_generate_test_produces_output(self) -> None:
        from ai.claude.tcl_ai import (
            _build_test_script,
            _extract_irule_commands,
            _extract_object_refs,
            _extract_variables,
            _infer_profiles,
        )
        from core.commands.registry.namespace_data import order_events_for_file

        events = order_events_for_file(self.SAMPLE_IRULE)
        profiles = _infer_profiles(events)
        commands = _extract_irule_commands(self.SAMPLE_IRULE)
        objects = _extract_object_refs(self.SAMPLE_IRULE, commands)
        variables = _extract_variables(self.SAMPLE_IRULE)

        script = _build_test_script(
            basename="test.tcl",
            source=self.SAMPLE_IRULE,
            ordered_events=events,
            profiles=profiles,
            commands_used=commands,
            objects=objects,
            variables=variables,
        )

        assert "::orch::configure_tests" in script
        assert "::orch::test" in script
        assert "::orch::run_http_request" in script
        assert "::orch::done" in script

    def test_profiles_inferred_correctly(self) -> None:
        from ai.claude.tcl_ai import _infer_profiles

        assert _infer_profiles(["HTTP_REQUEST"]) == ["TCP", "HTTP"]
        assert _infer_profiles(["CLIENT_ACCEPTED"]) == ["TCP"]
        assert _infer_profiles(["DNS_REQUEST"]) == ["UDP", "DNS"]
        assert _infer_profiles(["CLIENTSSL_HANDSHAKE", "HTTP_REQUEST"]) == [
            "TCP",
            "CLIENTSSL",
            "HTTP",
        ]

    def test_extract_commands(self) -> None:
        from ai.claude.tcl_ai import _extract_irule_commands

        commands = _extract_irule_commands(self.SAMPLE_IRULE)
        assert "HTTP::host" in commands
        assert "pool" in commands
        assert "class" in commands
        assert "reject" in commands
        assert "log" in commands

    def test_extract_object_refs(self) -> None:
        from ai.claude.tcl_ai import _extract_irule_commands, _extract_object_refs

        commands = _extract_irule_commands(self.SAMPLE_IRULE)
        objects = _extract_object_refs(self.SAMPLE_IRULE, commands)
        assert "api_pool" in objects["pools"]
        assert "web_pool" in objects["pools"]
        assert "allowed_hosts" in objects["datagroups"]

    def test_generated_script_has_pool_setup(self) -> None:
        from ai.claude.tcl_ai import (
            _build_test_script,
            _extract_irule_commands,
            _extract_object_refs,
            _extract_variables,
            _infer_profiles,
        )
        from core.commands.registry.namespace_data import order_events_for_file

        events = order_events_for_file(self.SAMPLE_IRULE)
        profiles = _infer_profiles(events)
        commands = _extract_irule_commands(self.SAMPLE_IRULE)
        objects = _extract_object_refs(self.SAMPLE_IRULE, commands)
        variables = _extract_variables(self.SAMPLE_IRULE)

        script = _build_test_script(
            basename="test.tcl",
            source=self.SAMPLE_IRULE,
            ordered_events=events,
            profiles=profiles,
            commands_used=commands,
            objects=objects,
            variables=variables,
        )

        assert "api_pool" in script
        assert "web_pool" in script
        assert "allowed_hosts" in script
        assert "assert_that pool_selected" in script
        # CFG-informed generation produces reject assertion for the else path
        assert "reject was_called" in script


# Multi-TMM simulation tests


@skip_no_tkinter
class TestMultiTMMSimulation:
    """Integration tests for multi-TMM static variable isolation.

    On real BIG-IP, each TMM core maintains its own copy of static::
    variables (RULE_INIT fires independently per TMM).  The table
    command is CMP-shared.  These tests verify the framework models
    this correctly so users can find CMP-related bugs.
    """

    @staticmethod
    def _eval(script: str) -> str:
        import asyncio

        async def _run() -> str:
            async with IruleTestSession(backend="inprocess", profiles=["TCP", "HTTP"]) as session:
                await session.add_pool("web_pool", ["10.0.1.1:80"])
                resp = await session._send({"cmd": "eval", "script": script})
                return resp.get("result", "")

        return asyncio.new_event_loop().run_until_complete(_run())

    def test_tmm_select_requires_tmm_count(self) -> None:
        """tmm_select errors without -tmm_count configured."""
        result = self._eval("""
            if {[catch {::orch::tmm_select 0} err]} {
                return "error: $err"
            }
            return "no_error"
        """)
        assert "error:" in result
        assert "tmm_select requires" in result

    def test_tmm_select_range_check(self) -> None:
        """tmm_select errors for out-of-range TMM ID."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 4
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST { pool web_pool }
            }
            if {[catch {::orch::tmm_select 5} err]} {
                return "error: $err"
            }
            return "no_error"
        """)
        assert "error:" in result
        assert "out of range" in result

    def test_static_vars_isolated_per_tmm(self) -> None:
        """static:: variables are independent per-TMM."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 2
            ::orch::load_irule {
                when RULE_INIT { set static::counter 0 }
                when HTTP_REQUEST {
                    set static::counter [expr {$static::counter + 1}]
                    pool web_pool
                }
            }

            # TMM 0: one request -> counter = 1
            ::orch::tmm_select 0
            ::orch::run_http_request -host a.example.com

            # TMM 1: one request -> counter = 1 (independent RULE_INIT)
            ::orch::tmm_select 1
            ::orch::run_http_request -host b.example.com

            # TMM 0 again: another request -> counter = 2
            ::orch::tmm_select 0
            ::orch::run_http_request -host c.example.com

            # Read TMM 0's counter (should be 2)
            set t0 [::orch::tmm_get_static 0 counter]
            # Read TMM 1's counter (should be 1)
            set t1 [::orch::tmm_get_static 1 counter]
            return "$t0,$t1"
        """)
        assert result == "2,1"

    def test_rule_init_fires_per_tmm(self) -> None:
        """RULE_INIT fires independently when switching to a new TMM."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 3
            ::orch::load_irule {
                when RULE_INIT { set static::init_ran 1 }
                when HTTP_REQUEST { pool web_pool }
            }

            # Only select TMMs 0 and 2 (skip 1)
            ::orch::tmm_select 0
            set t0 [::orch::tmm_get_static 0 init_ran]

            ::orch::tmm_select 2
            set t2 [::orch::tmm_get_static 2 init_ran]

            # TMM 1 was never selected -- should have no statics
            set t1 [::orch::tmm_get_static 1 init_ran]

            return "$t0,$t1,$t2"
        """)
        assert result == "1,,1"

    def test_table_shared_across_tmms(self) -> None:
        """table command (CMP) is shared across all TMMs."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 2
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST {
                    table set rate_limit [IP::client_addr] 1 300
                    pool web_pool
                }
            }

            # TMM 0 sets a table entry
            ::orch::tmm_select 0
            ::orch::run_http_request -host a.example.com

            # TMM 1 should see the same table entry (CMP-shared)
            ::orch::tmm_select 1
            set val [table lookup rate_limit 10.0.0.1]
            return $val
        """)
        assert result == "1"

    def test_tmm_ids_and_current(self) -> None:
        """tmm_ids returns all TMM IDs, tmm_current returns active."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 4
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST { pool web_pool }
            }
            set ids [::orch::tmm_ids]
            ::orch::tmm_select 2
            set cur [::orch::tmm_current]
            return "[llength $ids],$cur"
        """)
        assert result == "4,2"

    def test_fluent_assert_tmm_var(self) -> None:
        """Fluent DSL assert_that tmm_var works."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 2
            ::orch::load_irule {
                when RULE_INIT { set static::mode "active" }
                when HTTP_REQUEST {
                    set static::mode "processing"
                    pool web_pool
                }
            }

            # TMM 0: RULE_INIT + request -> mode = "processing"
            ::orch::tmm_select 0
            ::orch::run_http_request -host a.example.com

            # TMM 1: only RULE_INIT -> mode = "active"
            ::orch::tmm_select 1

            # Assert cross-TMM state
            set r1 [::orch::assert_that tmm_var 0 mode equals processing]
            set r2 [::orch::assert_that tmm_var 1 mode equals active]
            return "$r1,$r2"
        """)
        assert result == "1,1"

    def test_connection_state_per_tmm(self) -> None:
        """Connection state is per-TMM (not shared)."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 2
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST {
                    HTTP::header insert X-TMM [::orch::tmm_current]
                    pool web_pool
                }
            }

            # TMM 0: send a request
            ::orch::tmm_select 0
            ::orch::run_http_request -host a.example.com
            set hdr0 [::state::http::request::header get X-TMM]

            # TMM 1: send a different request
            ::orch::tmm_select 1
            ::orch::run_http_request -host b.example.com
            set hdr1 [::state::http::request::header get X-TMM]

            return "$hdr0,$hdr1"
        """)
        assert result == "0,1"

    def test_reset_reinitializes_tmm_slots(self) -> None:
        """reset clears all TMM slots for clean test isolation."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 2
            ::orch::load_irule {
                when RULE_INIT { set static::counter 0 }
                when HTTP_REQUEST {
                    set static::counter [expr {$static::counter + 1}]
                    pool web_pool
                }
            }

            # Use TMM 0
            ::orch::tmm_select 0
            ::orch::run_http_request -host a.example.com
            set before [::orch::tmm_get_static 0 counter]

            # Reset everything (as ::orch::test does between tests)
            ::orch::reset
            ::orch::init
            ::orch::configure -profiles {TCP HTTP}

            # Re-configure TMM mode and use TMM 0 again
            ::orch::configure_tests -tmm_count 2
            ::orch::load_irule {
                when RULE_INIT { set static::counter 0 }
                when HTTP_REQUEST {
                    set static::counter [expr {$static::counter + 1}]
                    pool web_pool
                }
            }
            ::orch::tmm_select 0
            ::orch::run_http_request -host a.example.com
            set after [::orch::tmm_get_static 0 counter]

            return "$before,$after"
        """)
        assert result == "1,1"

    def test_fakecmp_hash_deterministic(self) -> None:
        """fakeCMP hash produces same TMM for same connection tuple."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 4
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST { pool web_pool }
            }
            set t1 [::orch::_fakecmp_hash 10.0.0.1 12345 192.168.1.100 443]
            set t2 [::orch::_fakecmp_hash 10.0.0.1 12345 192.168.1.100 443]
            set t3 [::orch::_fakecmp_hash 10.0.0.2 12345 192.168.1.100 443]
            return "$t1,$t2,$t3"
        """)
        parts = result.split(",")
        assert parts[0] == parts[1]
        for p in parts:
            assert 0 <= int(p) < 4

    def test_fakecmp_hash_distributes(self) -> None:
        """fakeCMP hash distributes different clients across TMMs."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 4
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST { pool web_pool }
            }
            set seen [dict create]
            for {set i 1} {$i <= 20} {incr i} {
                set tmm [::orch::_fakecmp_hash 10.0.0.$i [expr {10000 + $i}] 192.168.1.100 443]
                dict set seen $tmm 1
            }
            return [dict size $seen]
        """)
        assert int(result) >= 2

    def test_fakecmp_auto_tmm_select(self) -> None:
        """Auto TMM mode selects TMM via fakeCMP hash."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 4 -tmm_select auto
            ::orch::load_irule {
                when RULE_INIT { set static::hits 0 }
                when HTTP_REQUEST {
                    incr static::hits
                    pool web_pool
                }
            }

            # Two requests from same client: same TMM
            ::orch::run_http_request -host a.example.com
            set tmm_a [::orch::tmm_current]
            ::orch::run_http_request -host b.example.com
            set tmm_b [::orch::tmm_current]

            # Change client address: may hit different TMM
            ::orch::configure -client_addr 10.0.0.99 -client_port 54321
            ::orch::run_http_request -host c.example.com
            set tmm_c [::orch::tmm_current]

            return "$tmm_a,$tmm_b,$tmm_c"
        """)
        parts = result.split(",")
        assert parts[0] == parts[1]
        for p in parts:
            assert 0 <= int(p) < 4

    def test_fakecmp_which_tmm_with_args(self) -> None:
        """fakecmp_which_tmm returns TMM for explicit 4-tuple."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 4
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST { pool web_pool }
            }
            set t1 [::orch::fakecmp_which_tmm 10.0.0.1 12345 192.168.1.100 443]
            set t2 [::orch::fakecmp_which_tmm 10.0.0.1 12345 192.168.1.100 443]
            return "$t1,$t2"
        """)
        parts = result.split(",")
        assert parts[0] == parts[1]
        assert 0 <= int(parts[0]) < 4

    def test_fakecmp_which_tmm_no_args(self) -> None:
        """fakecmp_which_tmm with no args uses current config."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 4
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST { pool web_pool }
            }
            ::orch::configure -client_addr 10.0.0.5 -client_port 22222
            set tmm [::orch::fakecmp_which_tmm]
            return $tmm
        """)
        assert 0 <= int(result) < 4

    def test_fakecmp_suggest_sources(self) -> None:
        """fakecmp_suggest_sources returns sources for each TMM."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 4
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST { pool web_pool }
            }
            set plan [::orch::fakecmp_suggest_sources -count 2]
            # Verify each TMM has entries
            set ok 1
            for {set t 0} {$t < 4} {incr t} {
                set sources [dict get $plan $t]
                if {[llength $sources] < 2} { set ok 0 }
            }
            return $ok
        """)
        assert result == "1"

    def test_fakecmp_suggest_sources_verifies_hash(self) -> None:
        """Suggested sources actually hash to the expected TMM."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 4
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST { pool web_pool }
            }
            set plan [::orch::fakecmp_suggest_sources -count 1]
            set ok 1
            for {set t 0} {$t < 4} {incr t} {
                set sources [dict get $plan $t]
                set addr [lindex $sources 0]
                set port [lindex $sources 1]
                set actual [::orch::fakecmp_which_tmm $addr $port 192.168.1.100 443]
                if {$actual != $t} { set ok 0 }
            }
            return $ok
        """)
        assert result == "1"

    def test_fakecmp_plan_output(self) -> None:
        """fakecmp_plan returns human-readable distribution text."""
        result = self._eval("""
            ::orch::configure_tests -tmm_count 3
            ::orch::load_irule {
                when RULE_INIT { set static::x 0 }
                when HTTP_REQUEST { pool web_pool }
            }
            return [::orch::fakecmp_plan -count 1]
        """)
        assert "fakeCMP distribution plan (3 TMMs):" in result
        assert "TMM 0:" in result
        assert "TMM 1:" in result
        assert "TMM 2:" in result

    def test_fakecmp_python_tcl_parity(self) -> None:
        """Python MCP fakeCMP hash matches Tcl _fakecmp_hash over a corpus."""

        # Python implementation (same as in tcl_mcp_server.py)
        def py_fakecmp(
            src_addr: str, src_port: int, dst_addr: str, dst_port: int, tmm_count: int
        ) -> int:
            h = 0
            for octet in src_addr.split("."):
                h = (h * 31 + int(octet)) & 0x7FFFFFFF
            h = (h * 31 + src_port) & 0x7FFFFFFF
            for octet in dst_addr.split("."):
                h = (h * 31 + int(octet)) & 0x7FFFFFFF
            h = (h * 31 + dst_port) & 0x7FFFFFFF
            return h % tmm_count

        # Build a corpus of test tuples
        corpus = []
        for octet in range(1, 51):
            for port in [10001, 12345, 54321, 80, 443]:
                corpus.append((f"10.0.0.{octet}", port, "192.168.1.100", 443))
        # Edge cases
        corpus.extend(
            [
                ("0.0.0.0", 0, "0.0.0.0", 0),
                ("255.255.255.255", 65535, "255.255.255.255", 65535),
                ("1.2.3.4", 1, "5.6.7.8", 2),
                ("172.16.0.1", 32768, "10.10.10.10", 8080),
            ]
        )

        for tmm_count in [2, 3, 4, 8]:
            # Compute Python results
            py_results = {}
            for src_addr, src_port, dst_addr, dst_port in corpus:
                key = f"{src_addr}:{src_port}->{dst_addr}:{dst_port}"
                py_results[key] = py_fakecmp(src_addr, src_port, dst_addr, dst_port, tmm_count)

            # Build Tcl script to compute same corpus
            tcl_lines = [
                f"::orch::configure_tests -tmm_count {tmm_count}",
                "::orch::load_irule {",
                "    when RULE_INIT { set static::x 0 }",
                "    when HTTP_REQUEST { pool web_pool }",
                "}",
                "set results [list]",
            ]
            for src_addr, src_port, dst_addr, dst_port in corpus:
                tcl_lines.append(
                    f"lappend results [::orch::_fakecmp_hash {src_addr} {src_port} {dst_addr} {dst_port}]"
                )
            tcl_lines.append("return [join $results ,]")
            tcl_script = "\n".join(tcl_lines)

            tcl_result = self._eval(tcl_script)
            tcl_tmms = [int(x) for x in tcl_result.split(",")]

            # Compare every tuple
            for i, (src_addr, src_port, dst_addr, dst_port) in enumerate(corpus):
                key = f"{src_addr}:{src_port}->{dst_addr}:{dst_port}"
                assert py_results[key] == tcl_tmms[i], (
                    f"Parity mismatch for {key} with {tmm_count} TMMs: "
                    f"Python={py_results[key]} Tcl={tcl_tmms[i]}"
                )


class TestCFGPathExtraction:
    """Tests for CFG-informed test path extraction."""

    def test_extract_simple_if_else(self) -> None:
        """Extract paths from a simple if/else iRule."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    if { [HTTP::host] eq "good.com" } {
        pool good_pool
    } else {
        reject
    }
}
"""
        paths = _extract_test_paths(source)
        assert len(paths) == 2
        # Paths are sorted by priority: reject (high) before pool (normal)
        reject_path = next(p for p in paths if p["action"]["command"] == "reject")
        pool_path = next(p for p in paths if p["action"]["command"] == "pool")
        # Pool path
        assert pool_path["event"] == "HTTP_REQUEST"
        assert pool_path["action"]["args"] == ["good_pool"]
        assert len(pool_path["conditions"]) == 1
        assert pool_path["conditions"][0]["kind"] == "if"
        assert pool_path["conditions"][0]["branch"] == "true"
        # Reject path
        assert reject_path["conditions"][0]["branch"] == "else"
        assert reject_path["priority"] == "high"
        # Both have questions
        assert len(pool_path["questions"]) >= 1
        assert len(reject_path["questions"]) >= 1

    def test_extract_switch_arms(self) -> None:
        """Extract paths from a switch statement."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    switch -glob [HTTP::uri] {
        "/api/*" { pool api_pool }
        "/static/*" { pool static_pool }
        default { pool web_pool }
    }
}
"""
        paths = _extract_test_paths(source)
        assert len(paths) == 3
        commands = [p["action"]["command"] for p in paths]
        assert all(c == "pool" for c in commands)
        pools = [p["action"]["args"][0] for p in paths]
        assert "api_pool" in pools
        assert "static_pool" in pools
        assert "web_pool" in pools
        # Each path should have a switch condition
        for p in paths:
            assert any(c["kind"] == "switch" for c in p["conditions"])

    def test_extract_nested_conditions(self) -> None:
        """Extract paths from nested if + switch."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    if { [HTTP::host] eq "api.example.com" } {
        pool api_pool
    } else {
        switch -glob [HTTP::uri] {
            "/admin/*" { reject }
            default { pool web_pool }
        }
    }
}
"""
        paths = _extract_test_paths(source)
        assert len(paths) == 3
        # Find paths by action (order-independent due to priority sorting)
        api_pool = next(
            p
            for p in paths
            if p["action"]["command"] == "pool" and "api_pool" in p["action"].get("args", [])
        )
        reject_path = next(p for p in paths if p["action"]["command"] == "reject")
        web_pool = next(
            p
            for p in paths
            if p["action"]["command"] == "pool" and "web_pool" in p["action"].get("args", [])
        )
        # api_pool path: host match -> pool api_pool
        assert len(api_pool["conditions"]) == 1
        # reject path: else + /admin/* -> reject
        assert len(reject_path["conditions"]) == 2
        assert reject_path["conditions"][0]["branch"] == "else"
        assert reject_path["conditions"][1]["kind"] == "switch"
        # web_pool path: else + default -> pool web_pool
        assert web_pool["conditions"][1]["pattern"] == "default"

    def test_extract_multiple_events(self) -> None:
        """Paths from multiple event handlers."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    pool web_pool
}
when HTTP_RESPONSE {
    HTTP::header remove Server
}
"""
        paths = _extract_test_paths(source)
        events = {p["event"] for p in paths}
        assert "HTTP_REQUEST" in events

    def test_extract_empty_irule(self) -> None:
        """Empty source returns no paths."""
        from ai.claude.tcl_ai import _extract_test_paths

        paths = _extract_test_paths("")
        assert paths == []

    def test_extract_no_actions(self) -> None:
        """iRule with only variable assignments returns no paths."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when RULE_INIT {
    set static::debug 0
}
"""
        paths = _extract_test_paths(source)
        assert paths == []

    def test_path_labels_are_readable(self) -> None:
        """Path labels contain event name and action."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    pool web_pool
}
"""
        paths = _extract_test_paths(source)
        assert len(paths) == 1
        label = paths[0]["path_label"]
        assert "HTTP_REQUEST" in label
        assert "pool" in label

    def test_gen_cfg_test_cases_produces_valid_output(self) -> None:
        """CFG test generation produces ::orch::test blocks."""
        from ai.claude.tcl_ai import _build_all_test_blocks, _extract_test_paths
        from ai.claude.templates.render import render_test_case

        source = """
when HTTP_REQUEST {
    if { [HTTP::host] eq "good.com" } {
        pool good_pool
    } else {
        reject
    }
}
"""
        paths = _extract_test_paths(source)
        blocks = _build_all_test_blocks(
            "myirule",
            paths,
            [],
            ["TCP", "HTTP"],
            [],
            {"pools": ["good_pool"]},
            render_test_case,
        )
        output = "\n".join(blocks)
        assert "::orch::test" in output
        assert "pool_selected" in output
        assert "reject" in output
        assert "cfg-1.0" in output
        assert "cfg-2.0" in output

    def test_cfg_pattern_to_value(self) -> None:
        """Switch patterns are converted to concrete test values."""
        from ai.claude.tcl_ai import _cfg_pattern_to_value

        assert _cfg_pattern_to_value("/api/*") == "/api/test"
        assert _cfg_pattern_to_value("example.com") == "example.com"
        assert _cfg_pattern_to_value("default") is None
        assert _cfg_pattern_to_value("/file?.txt") == "/filex.txt"

    def test_priority_sorting_order(self) -> None:
        """High-priority paths (security) are sorted before normal and low."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    switch -glob [HTTP::uri] {
        "/admin/*" { reject }
        "/api/*"   { pool api_pool }
        default    { log local0. "miss" }
    }
}
"""
        paths = _extract_test_paths(source)
        priorities = [p["priority"] for p in paths]
        # reject -> high, pool -> normal, log -> low
        assert priorities.index("high") < priorities.index("normal")
        if "low" in priorities:
            assert priorities.index("normal") <= priorities.index("low")

    def test_question_generation_pool(self) -> None:
        """Pool actions generate expected_pool questions."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    if { [HTTP::host] eq "api.example.com" } {
        pool api_pool
    }
}
"""
        paths = _extract_test_paths(source)
        assert len(paths) == 1
        qs = paths[0]["questions"]
        pool_q = next((q for q in qs if q["field"] == "expected_pool"), None)
        assert pool_q is not None
        assert "api_pool" in pool_q["question"]
        assert pool_q["suggested"] == "api_pool"

    def test_question_generation_reject(self) -> None:
        """Reject actions generate expected_reject questions."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    if { [HTTP::uri] starts_with "/blocked" } {
        reject
    }
}
"""
        paths = _extract_test_paths(source)
        reject_path = next(p for p in paths if p["action"]["command"] == "reject")
        qs = reject_path["questions"]
        reject_q = next((q for q in qs if q["field"] == "expected_reject"), None)
        assert reject_q is not None
        assert reject_q["suggested"] == "yes"

    def test_question_generation_else_fallback(self) -> None:
        """Else branches generate fallback_input questions."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    if { [HTTP::host] eq "good.com" } {
        pool good_pool
    } else {
        reject
    }
}
"""
        paths = _extract_test_paths(source)
        reject_path = next(p for p in paths if p["action"]["command"] == "reject")
        fallback_q = next(
            (q for q in reject_path["questions"] if q["field"] == "fallback_input"),
            None,
        )
        assert fallback_q is not None
        assert fallback_q["suggested"] is None

    def test_question_generation_switch_default(self) -> None:
        """Switch default branches generate default_input questions."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    switch -glob [HTTP::uri] {
        "/api/*" { pool api_pool }
        default  { pool fallback_pool }
    }
}
"""
        paths = _extract_test_paths(source)
        default_path = next(p for p in paths if "fallback_pool" in p["action"].get("args", []))
        default_q = next(
            (q for q in default_path["questions"] if q["field"] == "default_input"),
            None,
        )
        assert default_q is not None

    def test_condition_summary_readable(self) -> None:
        """_build_condition_summary produces readable text."""
        from ai.claude.tcl_ai import _build_condition_summary

        conditions = [
            {"kind": "if", "condition": '[HTTP::host] eq "api.com"', "branch": "true"},
            {"kind": "switch", "subject": "[HTTP::uri]", "pattern": "/api/*"},
        ]
        summary = _build_condition_summary(conditions)
        assert "[HTTP::host]" in summary
        assert "matches '/api/*'" in summary

    def test_taint_warnings_field_present(self) -> None:
        """All paths include a taint_warnings list (possibly empty)."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    pool web_pool
}
"""
        paths = _extract_test_paths(source)
        assert len(paths) == 1
        assert "taint_warnings" in paths[0]
        assert isinstance(paths[0]["taint_warnings"], list)

    def test_priority_elevation_for_tainted_conditions(self) -> None:
        """Paths with HTTP::uri in conditions get elevated from low to normal."""
        from ai.claude.tcl_ai import _extract_test_paths

        source = """
when HTTP_REQUEST {
    if { [HTTP::uri] starts_with "/debug" } {
        log local0. "debug hit"
    }
}
"""
        paths = _extract_test_paths(source)
        # log is normally low priority, but HTTP::uri in condition elevates to normal
        log_path = next((p for p in paths if p["action"]["command"] == "log"), None)
        if log_path:
            assert log_path["priority"] in ("normal", "high")
