"""Tests for core.compiler.side_effects — command side-effect classification."""

from __future__ import annotations

import pytest

from core.compiler.side_effects import (
    CommandSideEffects,
    ConnectionSide,
    SideEffect,
    SideEffectTarget,
    StorageScope,
    StorageType,
    classify_side_effects,
)

# ---------------------------------------------------------------------------
# Enum smoke tests
# ---------------------------------------------------------------------------


class TestEnums:
    def test_storage_type_members(self) -> None:
        assert StorageType.SCALAR is not StorageType.LIST
        assert StorageType.DICT is not StorageType.ARRAY
        assert StorageType.UNKNOWN is not StorageType.SCALAR

    def test_storage_scope_has_f5_scopes(self) -> None:
        f5_scopes = {
            StorageScope.CONNECTION,
            StorageScope.STATIC,
            StorageScope.SESSION_TABLE,
            StorageScope.PERSISTENCE,
            StorageScope.DATA_GROUP,
        }
        assert all(s in StorageScope for s in f5_scopes)

    def test_connection_side_members(self) -> None:
        assert ConnectionSide.CLIENT is not ConnectionSide.SERVER
        assert ConnectionSide.BOTH is not ConnectionSide.GLOBAL
        assert ConnectionSide.NONE is not ConnectionSide.BOTH

    def test_side_effect_target_has_io_targets(self) -> None:
        io_targets = {
            SideEffectTarget.FILE_IO,
            SideEffectTarget.NETWORK_IO,
            SideEffectTarget.LOG_IO,
        }
        assert all(t in SideEffectTarget for t in io_targets)


# ---------------------------------------------------------------------------
# Dataclass tests
# ---------------------------------------------------------------------------


class TestSideEffect:
    def test_default_values(self) -> None:
        e = SideEffect(target=SideEffectTarget.VARIABLE)
        assert not e.reads
        assert not e.writes
        assert e.storage_type is StorageType.UNKNOWN
        assert e.scope is StorageScope.UNKNOWN
        assert e.connection_side is ConnectionSide.NONE
        assert e.namespace is None
        assert e.dialect is None
        assert e.key is None
        assert e.subtable is None

    def test_frozen(self) -> None:
        e = SideEffect(target=SideEffectTarget.VARIABLE, reads=True)
        with pytest.raises(AttributeError):
            e.reads = False  # type: ignore[misc]


class TestCommandSideEffects:
    def test_pure(self) -> None:
        cse = CommandSideEffects(pure=True, deterministic=True)
        assert cse.pure
        assert not cse.reads_any
        assert not cse.writes_any
        assert cse.targets == frozenset()

    def test_single_write(self) -> None:
        e = SideEffect(target=SideEffectTarget.VARIABLE, writes=True, key="x")
        cse = CommandSideEffects(effects=(e,))
        assert cse.writes_any
        assert not cse.reads_any
        assert SideEffectTarget.VARIABLE in cse.write_targets
        assert cse.affects_target(SideEffectTarget.VARIABLE)
        assert cse.writes_target(SideEffectTarget.VARIABLE)
        assert not cse.reads_target(SideEffectTarget.VARIABLE)

    def test_multiple_effects(self) -> None:
        e1 = SideEffect(target=SideEffectTarget.HTTP_HEADER, reads=True)
        e2 = SideEffect(target=SideEffectTarget.HTTP_HEADER, writes=True)
        cse = CommandSideEffects(effects=(e1, e2))
        assert cse.reads_any
        assert cse.writes_any
        assert cse.targets == frozenset({SideEffectTarget.HTTP_HEADER})

    def test_scopes_property(self) -> None:
        e1 = SideEffect(
            target=SideEffectTarget.VARIABLE,
            scope=StorageScope.PROC_LOCAL,
        )
        e2 = SideEffect(
            target=SideEffectTarget.SESSION_TABLE,
            scope=StorageScope.SESSION_TABLE,
        )
        cse = CommandSideEffects(effects=(e1, e2))
        assert cse.scopes == frozenset({StorageScope.PROC_LOCAL, StorageScope.SESSION_TABLE})

    def test_effects_in_scope(self) -> None:
        e1 = SideEffect(target=SideEffectTarget.VARIABLE, scope=StorageScope.PROC_LOCAL)
        e2 = SideEffect(target=SideEffectTarget.VARIABLE, scope=StorageScope.GLOBAL)
        cse = CommandSideEffects(effects=(e1, e2))
        local = cse.effects_in_scope(StorageScope.PROC_LOCAL)
        assert len(local) == 1
        assert local[0] is e1

    def test_effects_on_side(self) -> None:
        e1 = SideEffect(
            target=SideEffectTarget.POOL_SELECTION,
            connection_side=ConnectionSide.SERVER,
        )
        e2 = SideEffect(
            target=SideEffectTarget.HTTP_HEADER,
            connection_side=ConnectionSide.CLIENT,
        )
        cse = CommandSideEffects(effects=(e1, e2))
        server = cse.effects_on_side(ConnectionSide.SERVER)
        assert len(server) == 1
        assert server[0] is e1


# ---------------------------------------------------------------------------
# classify_side_effects tests
# ---------------------------------------------------------------------------


class TestClassifyPureCommands:
    def test_expr_is_pure(self) -> None:
        result = classify_side_effects("expr", ("1", "+", "2"))
        assert result.pure

    def test_llength_is_pure(self) -> None:
        result = classify_side_effects("llength", ("$x",))
        assert result.pure

    def test_string_length_is_pure(self) -> None:
        result = classify_side_effects("string", ("length", "hello"))
        assert result.pure


class TestClassifyVariableCommands:
    def test_set_write(self) -> None:
        result = classify_side_effects("set", ("x", "hello"))
        assert result.writes_any
        assert result.affects_target(SideEffectTarget.VARIABLE)
        assert len(result.effects) == 1
        e = result.effects[0]
        assert e.key == "x"
        assert e.writes
        assert e.scope is StorageScope.PROC_LOCAL
        assert e.storage_type is StorageType.SCALAR

    def test_set_read(self) -> None:
        result = classify_side_effects("set", ("x",))
        assert result.reads_any
        assert not result.writes_any
        e = result.effects[0]
        assert e.reads
        assert not e.writes

    def test_set_global_var(self) -> None:
        result = classify_side_effects("set", ("::myvar", "val"))
        e = result.effects[0]
        assert e.scope is StorageScope.GLOBAL
        assert e.namespace == "::"
        assert e.key == "::myvar"

    def test_set_namespace_var(self) -> None:
        result = classify_side_effects("set", ("::foo::bar::x", "val"))
        e = result.effects[0]
        assert e.scope is StorageScope.NAMESPACE
        assert e.namespace == "::foo::bar"

    def test_set_static_var_irules(self) -> None:
        result = classify_side_effects("set", ("static::counter", "0"), dialect="irules")
        e = result.effects[0]
        assert e.scope is StorageScope.STATIC
        assert e.connection_side is ConnectionSide.GLOBAL
        assert e.key == "static::counter"

    def test_set_static_var_f5_irules(self) -> None:
        result = classify_side_effects("set", ("static::counter", "0"), dialect="f5-irules")
        e = result.effects[0]
        assert e.scope is StorageScope.STATIC
        assert e.connection_side is ConnectionSide.GLOBAL
        assert e.key == "static::counter"

    def test_incr_reads_and_writes(self) -> None:
        result = classify_side_effects("incr", ("counter",))
        e = result.effects[0]
        assert e.reads
        assert e.writes
        assert e.storage_type is StorageType.SCALAR

    def test_lappend_is_list_type(self) -> None:
        result = classify_side_effects("lappend", ("mylist", "item"))
        e = result.effects[0]
        assert e.storage_type is StorageType.LIST
        assert e.reads
        assert e.writes


class TestClassifyDynamicBarriers:
    def test_eval_is_dynamic_barrier(self) -> None:
        result = classify_side_effects("eval", ("set x 1",))
        assert result.dynamic_barrier
        assert result.writes_any
        assert result.reads_any

    def test_uplevel_is_dynamic_barrier(self) -> None:
        result = classify_side_effects("uplevel", ("1", "set x 1"))
        assert result.dynamic_barrier


class TestClassifyTableCommand:
    def test_table_set(self) -> None:
        result = classify_side_effects("table", ("set", "mykey", "myval"), dialect="irules")
        assert result.writes_any
        e = result.effects[0]
        assert e.target is SideEffectTarget.SESSION_TABLE
        assert e.scope is StorageScope.SESSION_TABLE
        assert e.connection_side is ConnectionSide.BOTH

    def test_table_lookup(self) -> None:
        result = classify_side_effects("table", ("lookup", "mykey"), dialect="irules")
        e = result.effects[0]
        assert e.reads
        assert not e.writes
        assert e.target is SideEffectTarget.SESSION_TABLE
        assert e.scope is StorageScope.SESSION_TABLE


class TestClassifyF5Commands:
    def test_pool_selection(self) -> None:
        result = classify_side_effects("pool", ("mypool",), dialect="irules")
        assert result.writes_target(SideEffectTarget.POOL_SELECTION)
        e = result.effects[0]
        assert e.connection_side is ConnectionSide.SERVER

    def test_node_selection(self) -> None:
        result = classify_side_effects("node", ("10.0.0.1",), dialect="irules")
        assert result.writes_target(SideEffectTarget.NODE_SELECTION)

    def test_snat_selection(self) -> None:
        result = classify_side_effects("snat", ("automap",), dialect="irules")
        assert result.writes_target(SideEffectTarget.SNAT_SELECTION)

    def test_class_lookup(self) -> None:
        result = classify_side_effects("class", ("match", "-name", "myclass", "value"))
        e = result.effects[0]
        assert e.target is SideEffectTarget.DATA_GROUP
        assert e.scope is StorageScope.DATA_GROUP
        assert e.reads

    def test_persist_command(self) -> None:
        result = classify_side_effects("persist", ("source_addr",), dialect="irules")
        e = result.effects[0]
        assert e.target is SideEffectTarget.PERSISTENCE_TABLE
        assert e.scope is StorageScope.PERSISTENCE
        assert e.connection_side is ConnectionSide.CLIENT

    def test_session_add(self) -> None:
        result = classify_side_effects("session", ("add", "key", "val"), dialect="irules")
        e = result.effects[0]
        assert e.target is SideEffectTarget.PERSISTENCE_TABLE
        assert e.writes

    def test_session_lookup(self) -> None:
        result = classify_side_effects("session", ("lookup", "key"), dialect="irules")
        e = result.effects[0]
        assert e.reads
        assert not e.writes
        assert e.target is SideEffectTarget.PERSISTENCE_TABLE
        assert e.scope is StorageScope.PERSISTENCE


class TestClassifyProtocolNamespaceCommands:
    def test_http_header_read(self) -> None:
        result = classify_side_effects("HTTP::header", ("value", "Host"), dialect="irules")
        e = result.effects[0]
        assert e.namespace == "HTTP"
        assert e.reads

    def test_http_uri_normalized_getter_is_read_only(self) -> None:
        result = classify_side_effects("HTTP::uri", ("-normalized",), dialect="f5-irules")
        e = result.effects[0]
        assert e.target is SideEffectTarget.HTTP_URI
        assert e.reads
        assert not e.writes

    def test_dialect_propagation(self) -> None:
        result = classify_side_effects("set", ("x", "1"), dialect="irules")
        assert result.dialect == "irules"
        e = result.effects[0]
        assert e.dialect == "irules"

    def test_unknown_command_is_conservative(self) -> None:
        result = classify_side_effects("some_unknown_command", ("arg1",))
        assert result.reads_any
        assert result.writes_any


class TestClassifyWithRegistryHints:
    def test_command_level_hint_is_applied(self) -> None:
        result = classify_side_effects("HTTP2::disable", (), dialect="irules")
        assert result.writes_target(SideEffectTarget.HTTP2_STATE)
        effect = result.effects[0]
        assert effect.connection_side is ConnectionSide.BOTH
        assert effect.dialect == "irules"

    def test_subcommand_hint_overrides_command_level_heuristics(self) -> None:
        result = classify_side_effects(
            "HTTP::header", ("replace", "Host", "example.com"), dialect="irules"
        )
        assert len(result.effects) == 1
        effect = result.effects[0]
        assert effect.target is SideEffectTarget.HTTP_HEADER
        assert effect.writes

    def test_hint_precedence_beats_fallback_always_mutating(self) -> None:
        result = classify_side_effects("call", ("my_proc",), dialect="irules")
        assert result.reads_target(SideEffectTarget.PROC_DEFINITION)
        assert not result.writes_any

    def test_subcommand_hint_selected_when_provided_explicitly(self) -> None:
        result = classify_side_effects(
            "table", ("set", "k", "v"), subcommand="lookup", dialect="irules"
        )
        effect = result.effects[0]
        assert effect.target is SideEffectTarget.SESSION_TABLE
        assert effect.reads

    def test_hint_with_unspecified_dialect_preserves_hint_shape(self) -> None:
        result = classify_side_effects("HTTP2::disable", ())
        effect = result.effects[0]
        assert effect.target is SideEffectTarget.HTTP2_STATE
        assert effect.dialect is None


class TestDialectSpecificHints:
    def test_close_in_tcl_is_file_io(self) -> None:
        result = classify_side_effects("close", ("$fd",), dialect="tcl8.6")
        effect = result.effects[0]
        assert effect.target is SideEffectTarget.FILE_IO
        assert effect.connection_side is ConnectionSide.NONE

    def test_close_in_irules_is_connection_control(self) -> None:
        result = classify_side_effects("close", (), dialect="f5-irules")
        effect = result.effects[0]
        assert effect.target is SideEffectTarget.CONNECTION_CONTROL
        assert effect.connection_side is ConnectionSide.BOTH

    def test_file_io_does_not_kill_unknown_region(self) -> None:
        from core.compiler.side_effects import EffectRegion

        result = classify_side_effects("puts", ("hello",), dialect="tcl8.6")
        _reads, writes = result.to_effect_regions()
        assert writes is EffectRegion.NONE


# ---------------------------------------------------------------------------
# Conformance test matrix — key command families have non-UNKNOWN targets
# ---------------------------------------------------------------------------


class TestConformanceHintTargets:
    """Verify that key command families resolve to specific (non-UNKNOWN) targets
    via the registry hint path, ensuring hints are not dead metadata."""

    @pytest.mark.parametrize(
        "command, dialect, expected_target",
        [
            # iRules protocol namespaces
            ("HTTP2::disable", "irules", SideEffectTarget.HTTP2_STATE),
            ("HTTP2::push", "irules", SideEffectTarget.HTTP2_STATE),
            ("ASM::disable", "irules", SideEffectTarget.ASM_STATE),
            ("ASM::enable", "irules", SideEffectTarget.ASM_STATE),
            ("COMPRESS::enable", "irules", SideEffectTarget.STREAM_PROFILE),
            ("DOSL7::disable", "irules", SideEffectTarget.DOSL7_STATE),
            ("FLOW::create_related", "irules", SideEffectTarget.FLOW_STATE),
            ("FTP::enable", "irules", SideEffectTarget.FTP_STATE),
            ("ICAP::header", "irules", SideEffectTarget.ICAP_STATE),
            ("ISTATS::set", "irules", SideEffectTarget.ISTATS),
            ("LSN::address", "irules", SideEffectTarget.LSN_STATE),
            ("MESSAGE::proto", "irules", SideEffectTarget.MESSAGE_STATE),
            ("MR::message", "irules", SideEffectTarget.MESSAGE_STATE),
            ("REWRITE::enable", "irules", SideEffectTarget.STREAM_PROFILE),
            ("CLASSIFY::application", "irules", SideEffectTarget.CLASSIFICATION_STATE),
            ("CLASSIFICATION::app", "irules", SideEffectTarget.CLASSIFICATION_STATE),
            ("PROFILE::http", "irules", SideEffectTarget.BIGIP_CONFIG),
            ("X509::subject", "irules", SideEffectTarget.SSL_STATE),
            ("URI::path", "irules", SideEffectTarget.HTTP_URI),
            # Tcl core commands
            ("puts", None, SideEffectTarget.FILE_IO),
            ("open", None, SideEffectTarget.FILE_IO),
            ("socket", None, SideEffectTarget.NETWORK_IO),
            ("proc", None, SideEffectTarget.PROC_DEFINITION),
            ("namespace", None, SideEffectTarget.NAMESPACE_STATE),
            ("interp", None, SideEffectTarget.INTERP_STATE),
            ("package", None, SideEffectTarget.INTERP_STATE),
        ],
    )
    def test_command_has_specific_target(
        self, command: str, dialect: str | None, expected_target: SideEffectTarget
    ) -> None:
        result = classify_side_effects(command, (), dialect=dialect)
        targets = {e.target for e in result.effects}
        assert expected_target in targets, (
            f"{command} expected target {expected_target.name}, got {targets}"
        )


class TestHintCoverageNotDead:
    """Verify that commands with registry hints are actually consumed by the
    classifier, ensuring the migration is not dead metadata."""

    def test_hinted_irules_commands_return_non_unknown_effects(self) -> None:
        """Spot-check that hinted iRules commands produce structured effects."""
        hinted_commands = [
            "HTTP2::active",
            "HTTP2::stream",
            "ASM::fingerprint",
            "DATAGRAM::ip",
            "CRYPTO::hash",
            "STREAM::enable",
            "ONECONNECT::detach",
            "redirect",
        ]
        for cmd in hinted_commands:
            result = classify_side_effects(cmd, (), dialect="irules")
            assert result.effects, f"{cmd}: expected non-empty effects from hints"

    def test_hinted_tcl_commands_return_structured_effects(self) -> None:
        """Spot-check that hinted Tcl stdlib commands produce structured effects."""
        hinted_commands = [
            "puts",
            "gets",
            "open",
            "close",
            "file",
            "socket",
            "proc",
            "rename",
            "namespace",
            "set",
            "incr",
            "append",
            "unset",
        ]
        for cmd in hinted_commands:
            result = classify_side_effects(cmd, ("x",))
            assert result.effects, f"{cmd}: expected non-empty effects from hints"
