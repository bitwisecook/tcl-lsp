"""Tests for command metadata registry."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.commands.registry import REGISTRY
from core.commands.registry.info import lookup_command_info, lookup_event_info
from core.commands.registry.namespace_data import EVENT_PROPS
from core.commands.registry.runtime import SIGNATURES, configure_signatures


class TestRegistryStructure:
    def test_socket_is_registered_with_switches(self):
        switches = REGISTRY.switches("socket", "tcl8.6")
        assert "-server" in switches
        assert "-myaddr" in switches

    def test_irules_http_commands_are_dialect_scoped(self):
        assert REGISTRY.get("HTTP::header", "f5-irules") is not None
        assert REGISTRY.get("HTTP::header", "tcl8.6") is None

    def test_http_header_subcommand_values_are_registered(self):
        values = REGISTRY.argument_values("HTTP::header", 0, "f5-irules")
        labels = {v.value for v in values}
        assert "insert" in labels
        assert "replace" in labels

    def test_socket_server_option_has_hover_snippet(self):
        option = REGISTRY.option("socket", "-server", "tcl8.6")
        assert option is not None
        assert option.hover is not None
        assert "server mode" in option.hover.summary.lower()

    def test_registry_covers_all_tcl_core_signature_commands(self):
        for dialect in ("tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"):
            configure_signatures(dialect=dialect, extra_commands=[])
            missing = [name for name in SIGNATURES if REGISTRY.get(name, dialect) is None]
            assert missing == []

    def test_generated_doc_snippet_present_for_non_overridden_command(self):
        spec = REGISTRY.get("parray", "tcl8.6")
        assert spec is not None
        assert spec.hover is not None
        assert spec.hover.source

    def test_when_event_values_cover_event_props(self):
        values = REGISTRY.argument_values("when", 0, "f5-irules")
        labels = {value.value for value in values}
        for event_name in ("HTTP_REQUEST", "CLIENT_ACCEPTED", "SERVER_CONNECTED"):
            assert event_name in labels
        assert set(EVENT_PROPS).issubset(labels)

    def test_generated_irules_doc_snippet_present_for_non_overridden_command(self):
        spec = REGISTRY.get("ACCESS::acl", "f5-irules")
        assert spec is not None
        assert spec.hover is not None
        assert "clouddocs.f5.com" in spec.hover.source

    def test_curated_irules_override_wins_over_generated_data(self):
        spec = REGISTRY.get("HTTP::header", "f5-irules")
        assert spec is not None
        assert spec.hover is not None
        assert spec.hover.source == "https://clouddocs.f5.com/api/irules/HTTP__header.html"

    def test_registry_validation_metadata_is_available(self):
        socket_spec = REGISTRY.get("socket", "tcl8.6")
        assert socket_spec is not None
        socket_validation = REGISTRY.validation("socket", "tcl8.6")
        assert socket_validation is not None
        assert socket_validation.arity.min == 2
        assert socket_validation.arity.is_unlimited

        acl_spec = REGISTRY.get("ACCESS::acl", "f5-irules")
        assert acl_spec is not None
        acl_validation = REGISTRY.validation("ACCESS::acl", "f5-irules")
        assert acl_validation is not None
        # Generated command has arg_values for completion but flat validation.
        assert acl_validation.arity.is_unlimited


class TestControlFlowAndStartCmdTraits:
    """Tests for is_control_flow / needs_start_cmd trait queries."""

    def test_control_flow_includes_if_for_while(self):
        cf = REGISTRY.control_flow_commands()
        assert "if" in cf
        assert "for" in cf
        assert "while" in cf
        assert "foreach" in cf

    def test_control_flow_excludes_non_control(self):
        assert not REGISTRY.is_control_flow("set")
        assert not REGISTRY.is_control_flow("puts")

    def test_needs_start_cmd_includes_expr_break_continue(self):
        ns = REGISTRY.needs_start_cmd_commands()
        assert "expr" in ns
        assert "break" in ns
        assert "continue" in ns

    def test_needs_start_cmd_excludes_non_matching(self):
        assert not REGISTRY.is_needs_start_cmd("set")
        assert not REGISTRY.is_needs_start_cmd("puts")


class TestCommandsForEvent:
    """Tests for commands_for_event() and EventCommandSet."""

    def test_http_request_includes_http_header(self):
        es = REGISTRY.commands_for_event("f5-irules", "HTTP_REQUEST")
        assert "HTTP::header" in es.valid_commands

    def test_event_scoped_commands_populated(self):
        es = REGISTRY.commands_for_event("f5-irules", "HTTP_REQUEST")
        # HTTP::header has event_requires with HTTP profile, so it should
        # be in event_scoped_commands.
        assert es.event_scoped_commands
        # Event-neutral commands (like "set") should NOT be in event_scoped.
        assert "set" not in es.event_scoped_commands

    def test_out_of_event_for_rule_init(self):
        # HTTP::header requires an HTTP profile; RULE_INIT has no profile.
        es = REGISTRY.commands_for_event("f5-irules", "RULE_INIT")
        assert "HTTP::header" not in es.valid_commands
        assert "HTTP::header" in es.out_of_event_commands

    def test_unknown_event_demotes_event_requires_commands(self):
        es = REGISTRY.commands_for_event("f5-irules", "TOTALLY_FAKE_EVENT")
        # Commands with event_requires should be out-of-event for unknown events.
        assert "HTTP::header" in es.out_of_event_commands
        assert "HTTP::header" not in es.valid_commands

    def test_unknown_event_keeps_neutral_commands(self):
        es = REGISTRY.commands_for_event("f5-irules", "TOTALLY_FAKE_EVENT")
        # Event-neutral commands (like "set") should still be valid.
        assert "set" in es.valid_commands

    def test_excluded_events_respected(self):
        # TCP::rcv_scale excludes SERVER_INIT.
        spec = REGISTRY.get("TCP::rcv_scale", "f5-irules")
        if spec is not None and spec.excluded_events:
            excluded_event = spec.excluded_events[0]
            es = REGISTRY.commands_for_event("f5-irules", excluded_event)
            assert "TCP::rcv_scale" in es.out_of_event_commands


class TestCommandLegality:
    """Tests for command_legality() matrix."""

    def test_legality_is_legal_for_known_event(self):
        legality = REGISTRY.command_legality("f5-irules")
        assert legality.is_legal("HTTP_REQUEST", "HTTP::header")

    def test_legality_not_legal_in_rule_init(self):
        legality = REGISTRY.command_legality("f5-irules")
        assert not legality.is_legal("RULE_INIT", "HTTP::header")

    def test_legality_covers_all_event_props(self):
        legality = REGISTRY.command_legality("f5-irules")
        for event_name in EVENT_PROPS:
            # Every known event should have an entry.
            valid = legality.valid_commands(event_name)
            assert isinstance(valid, frozenset)

    def test_event_scoped_commands_have_at_least_one_legal_event(self):
        legality = REGISTRY.command_legality("f5-irules")
        for name in REGISTRY.command_names("f5-irules"):
            spec = REGISTRY.get(name, "f5-irules")
            if spec is None:
                continue
            if spec.event_requires is None and not spec.excluded_events:
                continue
            assert legality.events_for_command(name), f"{name} is legal in zero events"

    def test_http2_commands_are_legal_in_http_events(self):
        legality = REGISTRY.command_legality("f5-irules")
        for command in ("HTTP2::active", "HTTP2::concurrency", "HTTP2::stream", "HTTP2::version"):
            assert legality.is_legal("HTTP_REQUEST", command)
        assert legality.is_legal("MR_INGRESS", "HTTP2::active")
        assert legality.is_legal("MR_EGRESS", "HTTP2::active")
        assert legality.is_legal("MR_INGRESS", "HTTP2::version")


class TestLegalityParity:
    """Ensure legality answers are consistent across consumers."""

    def test_legality_matches_commands_for_event(self):
        """command_legality().is_legal() must agree with commands_for_event().valid_commands."""
        legality = REGISTRY.command_legality("f5-irules")
        for event_name in EVENT_PROPS:
            event_set = REGISTRY.commands_for_event("f5-irules", event_name)
            legality_valid = legality.valid_commands(event_name)
            assert event_set.valid_commands == legality_valid, f"Parity mismatch for {event_name}"
            legality_out = legality.out_of_event_commands(event_name)
            assert event_set.out_of_event_commands == legality_out, (
                f"Out-of-event parity mismatch for {event_name}"
            )

    def test_event_scoped_is_subset_of_valid(self):
        """event_scoped_commands must be a subset of valid_commands."""
        for event_name in EVENT_PROPS:
            event_set = REGISTRY.commands_for_event("f5-irules", event_name)
            assert event_set.event_scoped_commands <= event_set.valid_commands, (
                f"event_scoped not subset of valid for {event_name}"
            )

    def test_valid_and_out_of_event_are_disjoint(self):
        """valid_commands and out_of_event_commands must not overlap."""
        for event_name in list(EVENT_PROPS)[:10]:  # sample for speed
            event_set = REGISTRY.commands_for_event("f5-irules", event_name)
            overlap = event_set.valid_commands & event_set.out_of_event_commands
            assert not overlap, f"Overlap in {event_name}: {overlap}"


class TestEventPropsFlags:
    """Tests for deprecated/hot/common flags on EventProps."""

    def test_deprecated_events_derived_from_registry(self):
        from core.commands.registry.namespace_data import deprecated_events

        deps = deprecated_events()
        assert "ASM_REQUEST_VIOLATION" in deps
        assert "XML_EVENT" in deps
        assert "HTTP_CLASS_FAILED" in deps
        assert "HTTP_REQUEST" not in deps

    def test_hot_events_derived_from_registry(self):
        from core.commands.registry.namespace_data import hot_events

        hots = hot_events()
        assert "HTTP_REQUEST" in hots
        assert "HTTP_RESPONSE" in hots
        assert "CLIENT_ACCEPTED" in hots
        assert "RULE_INIT" not in hots

    def test_common_events_derived_from_registry(self):
        from core.commands.registry.namespace_data import common_events

        commons = common_events()
        assert len(commons) == 15
        assert "RULE_INIT" in commons
        assert "HTTP_REQUEST" in commons
        assert "DNS_REQUEST" in commons
        assert "DNS_RESPONSE" in commons


class TestRegistryInfoHelpers:
    def test_lookup_event_info_known_event(self):
        info = lookup_event_info("http_request", dialect="f5-irules")
        assert info.event == "HTTP_REQUEST"
        assert info.known
        assert info.valid_command_count >= 1
        assert info.side in {"client-side", "server-side", "client-side and server-side", "global"}

    def test_lookup_event_info_unknown_event(self):
        info = lookup_event_info("totally_fake_event", dialect="f5-irules")
        assert info.event == "TOTALLY_FAKE_EVENT"
        assert not info.known
        assert info.valid_commands == ()
        assert info.side == "unknown"

    def test_lookup_command_info_case_insensitive(self):
        info = lookup_command_info("http::uri", dialect="f5-irules")
        assert info.found
        assert info.command == "HTTP::uri"

    def test_lookup_command_info_not_found(self):
        info = lookup_command_info("definitely::not_a_command", dialect="f5-irules")
        assert not info.found
        assert info.command == "definitely::not_a_command"


class TestFormKindAndResolveForm:
    """Tests for FormKind enum, resolve_form(), and structured getter/setter forms."""

    def test_form_kind_enum_values(self):
        from core.commands.registry.models import FormKind

        assert FormKind.DEFAULT.value == "default"
        assert FormKind.GETTER.value == "getter"
        assert FormKind.SETTER.value == "setter"

    def test_resolve_form_single_form_returns_it(self):
        """When a command has only one form, resolve_form always returns it."""
        spec = REGISTRY.get("set", "tcl8.6")
        assert spec is not None
        form = spec.resolve_form(("x", "1"))
        assert form is not None

    def test_resolve_form_http_uri_getter(self):
        from core.commands.registry.models import FormKind

        spec = REGISTRY.get("HTTP::uri", "f5-irules")
        assert spec is not None
        form = spec.resolve_form(())
        assert form is not None
        assert form.kind is FormKind.GETTER
        assert form.pure is True
        assert form.mutator is False

    def test_resolve_form_http_uri_setter(self):
        from core.commands.registry.models import FormKind

        spec = REGISTRY.get("HTTP::uri", "f5-irules")
        assert spec is not None
        form = spec.resolve_form(("/new/path",))
        assert form is not None
        assert form.kind is FormKind.SETTER
        assert form.mutator is True
        assert form.pure is False

    def test_resolve_form_http_path_getter_setter(self):
        from core.commands.registry.models import FormKind

        spec = REGISTRY.get("HTTP::path", "f5-irules")
        assert spec is not None
        getter = spec.resolve_form(())
        setter = spec.resolve_form(("/new",))
        assert getter is not None
        assert setter is not None
        assert getter.kind is FormKind.GETTER
        assert setter.kind is FormKind.SETTER

    def test_resolve_form_http_query_getter_setter(self):
        from core.commands.registry.models import FormKind

        spec = REGISTRY.get("HTTP::query", "f5-irules")
        assert spec is not None
        getter = spec.resolve_form(())
        setter = spec.resolve_form(("a=b",))
        assert getter is not None
        assert setter is not None
        assert getter.kind is FormKind.GETTER
        assert setter.kind is FormKind.SETTER

    def test_resolve_form_http_host_getter_only(self):
        from core.commands.registry.models import FormKind

        spec = REGISTRY.get("HTTP::host", "f5-irules")
        assert spec is not None
        form = spec.resolve_form(())
        assert form is not None
        assert form.kind is FormKind.GETTER
        assert form.pure is True

    def test_resolve_form_returns_none_for_no_forms(self):
        from core.commands.registry.models import CommandSpec, HoverSnippet

        spec = CommandSpec(
            name="test",
            hover=HoverSnippet(summary="test"),
        )
        assert spec.resolve_form(()) is None

    def test_per_form_side_effect_hints(self):
        spec = REGISTRY.get("HTTP::uri", "f5-irules")
        assert spec is not None
        getter = spec.resolve_form(())
        setter = spec.resolve_form(("/new",))
        assert getter is not None
        assert setter is not None
        assert len(getter.side_effect_hints) == 1
        assert getter.side_effect_hints[0].reads is True
        assert getter.side_effect_hints[0].writes is False
        assert len(setter.side_effect_hints) == 1
        assert setter.side_effect_hints[0].reads is True
        assert setter.side_effect_hints[0].writes is True


class TestSubCommandResolveForm:
    """Tests for resolve_form() on SubCommand entries."""

    def test_http_cookie_version_getter(self):
        from core.commands.registry.models import FormKind

        spec = REGISTRY.get("HTTP::cookie", "f5-irules")
        assert spec is not None
        sub = spec.subcommands["version"]
        # Getter: 1 arg (the cookie name)
        form = sub.resolve_form(("myCookie",))
        assert form is not None
        assert form.kind is FormKind.GETTER
        assert form.pure is True

    def test_http_cookie_version_setter(self):
        from core.commands.registry.models import FormKind

        spec = REGISTRY.get("HTTP::cookie", "f5-irules")
        assert spec is not None
        sub = spec.subcommands["version"]
        # Setter: 2 args (name, value)
        form = sub.resolve_form(("myCookie", "1"))
        assert form is not None
        assert form.kind is FormKind.SETTER
        assert form.mutator is True

    def test_http_header_lws_getter(self):
        from core.commands.registry.models import FormKind

        spec = REGISTRY.get("HTTP::header", "f5-irules")
        assert spec is not None
        sub = spec.subcommands["lws"]
        form = sub.resolve_form(())
        assert form is not None
        assert form.kind is FormKind.GETTER
        assert form.pure is True

    def test_http_header_lws_setter(self):
        from core.commands.registry.models import FormKind

        spec = REGISTRY.get("HTTP::header", "f5-irules")
        assert spec is not None
        sub = spec.subcommands["lws"]
        form = sub.resolve_form(("enable",))
        assert form is not None
        assert form.kind is FormKind.SETTER
        assert form.mutator is True

    def test_subcommand_without_forms_returns_none(self):
        spec = REGISTRY.get("HTTP::header", "f5-irules")
        assert spec is not None
        sub = spec.subcommands["value"]
        assert sub.resolve_form(("Host",)) is None


class TestDeprecatedReplacementTypeSafe:
    """Tests for type-safe deprecated_replacement."""

    def test_class_ref_returns_name(self):
        spec = REGISTRY.get("http_uri", "f5-irules")
        assert spec is not None
        assert spec.deprecated_replacement_name == "HTTP::uri"
        assert not isinstance(spec.deprecated_replacement, str)

    def test_string_ref_passes_through(self):
        spec = REGISTRY.get("vlan_id", "f5-irules")
        assert spec is not None
        assert spec.deprecated_replacement_name == "VLAN::id"
        assert isinstance(spec.deprecated_replacement, str)

    def test_none_returns_none(self):
        spec = REGISTRY.get("set", "tcl8.6")
        assert spec is not None
        assert spec.deprecated_replacement_name is None

    def test_all_class_refs_resolve_to_registered_commands(self):
        """Every deprecated_replacement that is a class ref must point to a registered command."""
        for name, specs in REGISTRY.specs_by_name.items():
            for spec in specs:
                dr = spec.deprecated_replacement
                if dr is not None and not isinstance(dr, str):
                    replacement_name = dr.name
                    assert REGISTRY.get(replacement_name, "f5-irules") is not None, (
                        f"{name} -> {replacement_name} not registered"
                    )


class TestClassifySideEffectsFormAware:
    """Tests that classify_side_effects uses form resolution."""

    def test_http_uri_getter_is_pure(self):
        from core.compiler.side_effects import classify_side_effects

        result = classify_side_effects("HTTP::uri", (), dialect="f5-irules")
        assert result.pure is True

    def test_http_uri_setter_is_impure(self):
        from core.compiler.side_effects import classify_side_effects

        result = classify_side_effects("HTTP::uri", ("/new",), dialect="f5-irules")
        assert result.pure is False


class TestSideEffectHintsDialectFiltering:
    """Ensure side-effect hints are selected from the active dialect's spec."""

    def test_close_hints_differ_by_dialect(self):
        from core.compiler.side_effects import SideEffectTarget

        tcl_hints = REGISTRY.side_effect_hints("close", dialect="tcl8.6")
        irules_hints = REGISTRY.side_effect_hints("close", dialect="f5-irules")

        assert tcl_hints is not None
        assert irules_hints is not None
        assert tcl_hints[0].target is SideEffectTarget.FILE_IO
        assert irules_hints[0].target is SideEffectTarget.CONNECTION_CONTROL
