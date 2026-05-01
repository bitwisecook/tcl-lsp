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

    def test_lookup_event_info_dual_transport_is_serialised(self):
        info = lookup_event_info("client_accepted", dialect="f5-irules")
        assert info.transport == "tcp/udp"

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

    def test_lookup_command_info_inherits_namespace_profile_requirements(self):
        info = lookup_command_info("ACCESS::log", dialect="f5-irules")
        assert info.found
        assert not info.valid_in_any_event
        assert info.valid_events
        assert "HTTP_REQUEST" not in info.valid_events
        assert any(name.startswith("ACCESS_") for name in info.valid_events)

    def test_lookup_command_info_any_event_command(self):
        """RESOLV::lookup is valid in any event per BIG-IP manpage."""
        info = lookup_command_info("RESOLV::lookup", dialect="f5-irules")
        assert info.found
        assert info.valid_in_any_event
        assert "RULE_INIT" in info.valid_events
        assert "HTTP_REQUEST" in info.valid_events

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

    def test_http_header_lws_no_forms(self):
        """HTTP::header lws is a simple no-arg query (returns 0/1), no forms."""
        from core.commands.registry.signatures import Arity

        spec = REGISTRY.get("HTTP::header", "f5-irules")
        assert spec is not None
        sub = spec.subcommands["lws"]
        assert sub.arity == Arity(0, 0)
        assert sub.resolve_form(()) is None

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


class TestLazyDialectLoading:
    """Tests for the lazy dialect loading mechanism."""

    def _fresh_registry(self):
        from core.commands.registry.command_registry import CommandRegistry

        return CommandRegistry.build_default()

    def test_dialect_specs_not_in_default_registry(self):
        registry = self._fresh_registry()
        assert "HTTP::header" not in registry.specs_by_name
        assert "button" not in registry.specs_by_name
        assert "set" in registry.specs_by_name

    def test_load_dialect_specs_returns_true_on_first_load(self):
        registry = self._fresh_registry()
        assert registry.load_dialect_specs("f5-irules") is True

    def test_load_dialect_specs_returns_false_on_second_load(self):
        registry = self._fresh_registry()
        registry.load_dialect_specs("f5-irules")
        assert registry.load_dialect_specs("f5-irules") is False

    def test_load_unknown_dialect_is_noop(self):
        registry = self._fresh_registry()
        assert registry.load_dialect_specs("nonexistent") is False

    def test_get_auto_loads_dialect(self):
        registry = self._fresh_registry()
        assert "HTTP::header" not in registry.specs_by_name
        spec = registry.get("HTTP::header", "f5-irules")
        assert spec is not None
        assert "HTTP::header" in registry.specs_by_name

    def test_validation_auto_loads_dialect(self):
        registry = self._fresh_registry()
        val = registry.validation("ACCESS::acl", "f5-irules")
        assert val is not None

    def test_commands_for_event_auto_loads_dialect(self):
        registry = self._fresh_registry()
        es = registry.commands_for_event("f5-irules", "HTTP_REQUEST")
        assert "HTTP::header" in es.valid_commands

    def test_command_names_auto_loads_dialect(self):
        registry = self._fresh_registry()
        names = set(registry.command_names("f5-irules"))
        assert "HTTP::header" in names

    def test_command_legality_auto_loads_dialect(self):
        registry = self._fresh_registry()
        legality = registry.command_legality("f5-irules")
        assert legality.is_legal("HTTP_REQUEST", "HTTP::header")

    def test_tk_loaded_for_tcl_dialects(self):
        registry = self._fresh_registry()
        spec = registry.get("button", "tcl8.6")
        assert spec is not None

    def test_eda_loads_tk_and_sdc(self):
        registry = self._fresh_registry()
        registry.load_dialect_specs("synopsys-eda-tcl")
        assert "button" in registry.specs_by_name
        # SDC base commands should also be present.
        assert len(registry._loaded_loaders & {"tk", "sdc-base", "synopsys-eda-tcl"}) == 3


class TestRewriteAliasResolution:
    """Issue #246 — CFG-rewritten command names must resolve through the registry.

    The CFG lowering pass rewrites ``dict for`` / ``dict map`` to the
    qualified names ``::tcl::dict::for`` / ``::tcl::dict::map`` so they
    can be dispatched as opaque barriers.  Downstream consumers (SSA,
    optimisations, diagnostics) must be able to query the registry on
    the rewritten name and get the same role, body-kind, and arity
    information that the source ensemble form provides — otherwise each
    caller has to hardcode the correspondence (the regression that
    issues #234, #236, #243 all stemmed from).
    """

    def test_rewrite_alias_map_covers_dict_iteration(self):
        from core.commands.registry.runtime import resolve_rewrite_alias

        assert resolve_rewrite_alias("::tcl::dict::for") == ("dict", "for")
        assert resolve_rewrite_alias("::tcl::dict::map") == ("dict", "map")

    def test_rewrite_alias_is_derived_from_subcommand_property(self):
        from core.commands.registry import REGISTRY
        from core.commands.registry.runtime import resolve_rewrite_alias

        # The rewrite alias is derived from ``SubCommand.cfg_rewrite_name``
        # on the source spec, not a hardcoded table in ``runtime.py``.  The
        # corresponding subcommand entry must declare the qualified name
        # explicitly so a future ensemble rewrite is automatically visible
        # to every registry-driven query.
        dict_spec = REGISTRY.get_any("dict")
        assert dict_spec is not None
        for_sub = dict_spec.subcommands["for"]
        map_sub = dict_spec.subcommands["map"]
        assert for_sub.cfg_rewrite_name == "::tcl::dict::for"
        assert map_sub.cfg_rewrite_name == "::tcl::dict::map"
        # And those declarations must drive the runtime resolver.
        assert resolve_rewrite_alias(for_sub.cfg_rewrite_name) == ("dict", "for")
        assert resolve_rewrite_alias(map_sub.cfg_rewrite_name) == ("dict", "map")

    def test_exported_short_name_resolves_via_registry_property(self):
        from core.commands.registry import REGISTRY
        from core.commands.registry.runtime import BodyKind, body_kind_for_command

        # The bare ``test`` name only resolves to ``tcltest::test`` because
        # the latter declares it is exported from its source namespace.
        # The bare-name itself is derived from the qualified spec name —
        # not duplicated as a separate field — so a future spec at
        # ``foo::bar`` exporting ``bar`` automatically participates.
        spec = REGISTRY.get_any("tcltest::test")
        assert spec is not None
        assert spec.is_namespace_exported is True
        # ``body_kind_for_command`` resolves through the over-approximation
        # alias map built from ``is_namespace_exported`` declarations.
        assert (
            body_kind_for_command("test", ["name", "desc", "-body", "body"]) is BodyKind.STRUCTURAL
        )

    def test_rewrite_alias_unrecognised_returns_none(self):
        from core.commands.registry.runtime import resolve_rewrite_alias

        assert resolve_rewrite_alias("dict") is None
        assert resolve_rewrite_alias("::my::for") is None
        assert resolve_rewrite_alias("foreach") is None

    def test_loop_var_list_role_resolves_via_rewrite_alias(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        # Rewritten name: subcommand word already consumed, so the role
        # index returned matches the original argument layout 1-for-1.
        assert arg_indices_for_role(
            "::tcl::dict::for", ["k v", "$d", "body"], ArgRole.LOOP_VAR_LIST
        ) == {0}
        assert arg_indices_for_role(
            "::tcl::dict::map", ["k v", "$d", "body"], ArgRole.LOOP_VAR_LIST
        ) == {0}

    def test_body_role_resolves_via_rewrite_alias(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        assert arg_indices_for_role("::tcl::dict::for", ["k v", "$d", "body"], ArgRole.BODY) == {2}
        assert arg_indices_for_role("::tcl::dict::map", ["k v", "$d", "body"], ArgRole.BODY) == {2}

    def test_ensemble_form_and_rewrite_yield_matching_body_indices(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        # The ensemble-form ``dict for`` index is offset by one (subcommand
        # word at args[0]); the rewritten form has no subcommand word so
        # the body index decrements by one.  Both must reach the same
        # ``args`` element after applying the natural offset.
        ensemble_body = arg_indices_for_role("dict", ["for", "k v", "$d", "body"], ArgRole.BODY)
        rewritten_body = arg_indices_for_role(
            "::tcl::dict::for", ["k v", "$d", "body"], ArgRole.BODY
        )
        assert ensemble_body == {3}
        assert rewritten_body == {2}
        # In each case the body argument is the same final word.

    def test_unknown_qualified_name_does_not_alias(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        # ``::my::for`` is a user-defined namespaced proc — it must not
        # accidentally pick up ``dict for`` semantics.  This is the
        # specific failure mode the suffix-test approach (now removed)
        # would have triggered.
        assert (
            arg_indices_for_role("::my::for", ["k v", "$d", "body"], ArgRole.LOOP_VAR_LIST) == set()
        )
        assert arg_indices_for_role("::my::for", ["k v", "$d", "body"], ArgRole.BODY) == set()


class TestMultiRoleArguments:
    """Issues #246 / #252 — an argument can carry multiple roles at once.

    ``dict with`` / ``dict update`` arg 0 names a variable that is both
    written (the dict is rewritten when the body returns) and read (the
    body sees keys unpacked into local variables of the same name).  The
    resolver attaches both :class:`ArgRole.VAR_READ` and
    :class:`ArgRole.VAR_WRITE` to the same index as a ``frozenset``, so a
    query for either role hits.  An earlier design used a combined
    ``VAR_READ_WRITE`` role plus a subsumption table; this is now
    expressed directly by the frozenset value.
    """

    def test_dict_with_var_arg_is_read_and_written(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        args = ["with", "myDict", "body"]
        assert arg_indices_for_role("dict", args, ArgRole.VAR_READ) == {1}
        assert arg_indices_for_role("dict", args, ArgRole.VAR_WRITE) == {1}

    def test_dict_update_var_arg_is_read_and_written(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        args = ["update", "myDict", "k", "v", "body"]
        assert arg_indices_for_role("dict", args, ArgRole.VAR_READ) == {1}
        assert arg_indices_for_role("dict", args, ArgRole.VAR_WRITE) == {1}

    def test_dict_with_body_index_unaffected_by_var_role(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        args = ["with", "myDict", "body"]
        assert arg_indices_for_role("dict", args, ArgRole.BODY) == {2}

    def test_var_read_write_deprecated_alias(self):
        """``VAR_READ_WRITE`` is preserved as a frozenset alias for back-compat."""
        from core.commands.registry.signatures import VAR_READ_WRITE, ArgRole

        assert VAR_READ_WRITE == frozenset({ArgRole.VAR_READ, ArgRole.VAR_WRITE})


class TestResolveArgRoleMap:
    """``resolve_arg_role_map`` matches ``arg_indices_for_role`` semantics.

    The map shape lets callers see every role on every annotated index
    without four separate registry lookups, and includes the same
    special-case logic ``arg_indices_for_role`` applies (TclOO body
    detection and regexp/regsub PATTERN index).
    """

    def test_dict_with_returns_multi_role_set(self):
        from core.commands.registry.runtime import ArgRole, resolve_arg_role_map

        roles = resolve_arg_role_map("dict", ["with", "myDict", "body"])
        assert roles[1] == frozenset({ArgRole.VAR_READ, ArgRole.VAR_WRITE})
        assert roles[2] == frozenset({ArgRole.BODY})

    def test_subcommand_offset_is_applied(self):
        from core.commands.registry.runtime import ArgRole, resolve_arg_role_map

        roles = resolve_arg_role_map("dict", ["update", "d", "k", "v", "body"])
        # arg 0 is "update" (subcommand word); the variable is at arg 1.
        assert ArgRole.VAR_WRITE in roles[1]
        assert ArgRole.BODY in roles[4]

    def test_regexp_pattern_index_is_included(self):
        from core.commands.registry.runtime import ArgRole, resolve_arg_role_map

        # ``regexp pat string matchVar`` -- arg 0 is the pattern.
        roles = resolve_arg_role_map("regexp", ["pat", "string", "m"])
        assert ArgRole.PATTERN in roles.get(0, frozenset())
        # Capture variable role from the resolver still applies.
        assert ArgRole.VAR_WRITE in roles.get(2, frozenset())

    def test_oo_method_body_index_is_included(self):
        from core.commands.registry.runtime import ArgRole, resolve_arg_role_map

        # ``oo::define cls method name args body`` -- body is the last arg.
        roles = resolve_arg_role_map("oo::define", ["cls", "method", "m", "{}", "{body}"])
        assert ArgRole.BODY in roles.get(4, frozenset())


class TestBodyKind:
    """Issue #246 — :class:`BodyKind` distinguishes structural from inline bodies.

    ``proc`` / ``when`` / ``tcltest::test`` bodies are *structural* — the
    body is analysed as its own scope/handler and must not be scanned by
    SSA as part of the calling block's data flow.  ``catch`` / ``while``
    / ``dict for`` bodies are *inline* — they share variables with the
    caller and their references must be visible to the enclosing scan.

    :func:`body_kind_for_command` resolves through the same
    ``cfg_rewrite_name`` and ``is_namespace_exported`` maps used by role
    queries so SSA has one consistent answer per call site.
    """

    def test_proc_body_is_structural(self):
        from core.commands.registry.runtime import BodyKind, body_kind_for_command

        assert body_kind_for_command("proc", ["name", "args", "body"]) is BodyKind.STRUCTURAL

    def test_when_body_is_structural(self):
        from core.commands.registry import REGISTRY
        from core.commands.registry.runtime import BodyKind, body_kind_for_command

        # ``when`` is iRules-only; ensure the dialect is loaded so the
        # spec is visible to ``REGISTRY.get_any``.
        REGISTRY.load_dialect_specs("f5-irules")
        assert body_kind_for_command("when", ["HTTP_REQUEST", "body"]) is BodyKind.STRUCTURAL

    def test_tcltest_test_body_is_structural(self):
        from core.commands.registry.runtime import BodyKind, body_kind_for_command

        assert (
            body_kind_for_command("tcltest::test", ["name", "desc", "-body", "body"])
            is BodyKind.STRUCTURAL
        )

    def test_namespace_imported_test_resolves_to_structural(self):
        from core.commands.registry.runtime import BodyKind, body_kind_for_command

        # ``test`` after ``namespace import ::tcltest::test`` shares the
        # source spec's body kind.
        assert (
            body_kind_for_command("test", ["name", "desc", "-body", "body"]) is BodyKind.STRUCTURAL
        )

    def test_dict_iteration_bodies_are_inline(self):
        from core.commands.registry.runtime import BodyKind, body_kind_for_command

        # The dict iter body shares the caller's scope — variable
        # references inside it must be visible to the enclosing function.
        assert body_kind_for_command("::tcl::dict::for", ["k v", "$d", "body"]) is BodyKind.INLINE
        assert body_kind_for_command("::tcl::dict::map", ["k v", "$d", "body"]) is BodyKind.INLINE

    def test_unregistered_command_defaults_to_inline(self):
        from core.commands.registry.runtime import BodyKind, body_kind_for_command

        # Defensive default: unknown commands don't get treated as
        # structural bodies and have their args silently dropped.
        assert body_kind_for_command("::my::custom::cmd", ["body"]) is BodyKind.INLINE

    def test_extra_command_registration_shadows_exported_alias(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
            configure_signatures,
        )

        # A user-defined ``test`` command (registered via extra_commands)
        # must shadow the ``test`` → ``tcltest::test`` static over-
        # approximation.  Otherwise CFG/SSA/check passes would
        # misclassify a user proc named ``test`` as a structural-body
        # tcltest test.
        try:
            configure_signatures(dialect="tcl8.6", extra_commands=["test"])
            # Direct registration → no body, no alias spillover.
            assert body_kind_for_command("test", ["a", "b", "-body", "x"]) is BodyKind.INLINE
            assert arg_indices_for_role("test", ["a", "b", "-body", "x"], ArgRole.BODY) == set()
            # And the qualified ``tcltest::test`` is unaffected.
            assert (
                body_kind_for_command("tcltest::test", ["a", "b", "-body", "x"])
                is BodyKind.STRUCTURAL
            )
        finally:
            configure_signatures(dialect="tcl8.6", extra_commands=[])

    def test_bare_test_resolves_to_alias_when_no_direct_registration(self):
        from core.commands.registry.runtime import BodyKind, body_kind_for_command

        # With no ``extra_commands`` registration, the static alias still
        # applies — preserving the behaviour for tcltest-style packages
        # where the bare name is the conventional spelling.
        assert body_kind_for_command("test", ["a", "b", "-body", "x"]) is BodyKind.STRUCTURAL


class TestTraceAddVariableVarWrite:
    """Issue #249 — ``trace add variable`` resolves *name* to ``ArgRole.VAR_WRITE``.

    The trace body can fire on read/write/unset and rewrite *name*, so
    SSA must see *name* as a definition site.  Routing this through the
    registry's sub-sub-resolver (dispatching on ``args[1]`` of ``trace add``)
    replaces the literal ``stmt.command == "trace"`` check that previously
    lived in ``core/compiler/ssa.py``.
    """

    def test_add_variable_marks_name_as_var_write(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        # ``trace add variable name ops body`` — ``name`` is at index 2.
        args = ["add", "variable", "x", "write", "watcher"]
        assert arg_indices_for_role("trace", args, ArgRole.VAR_WRITE) == {2}

    def test_add_variable_var_write_for_each_op_variant(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        # The op (read/write/unset/array) doesn't change the role of name.
        for op in ("read", "write", "unset", "array", "rwu"):
            args = ["add", "variable", "x", op, "watcher"]
            assert arg_indices_for_role("trace", args, ArgRole.VAR_WRITE) == {2}, op

    def test_add_command_does_not_mark_name_as_var_write(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        # ``trace add command`` is about a command name, not a variable.
        args = ["add", "command", "mycmd", "rename", "onCmd"]
        assert arg_indices_for_role("trace", args, ArgRole.VAR_WRITE) == set()

    def test_add_execution_does_not_mark_name_as_var_write(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        # ``trace add execution`` is about a command name, not a variable.
        args = ["add", "execution", "mycmd", "enter", "onExec"]
        assert arg_indices_for_role("trace", args, ArgRole.VAR_WRITE) == set()

    def test_remove_variable_does_not_mark_name_as_var_write(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        # Removing a trace doesn't add a callback, so SSA shouldn't treat
        # the name as a (re-)definition.
        args = ["remove", "variable", "x", "write", "watcher"]
        assert arg_indices_for_role("trace", args, ArgRole.VAR_WRITE) == set()

    def test_info_variable_does_not_mark_name_as_var_write(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        # ``trace info`` only inspects.
        args = ["info", "variable", "x"]
        assert arg_indices_for_role("trace", args, ArgRole.VAR_WRITE) == set()

    def test_add_variable_truncated_args_yield_empty(self):
        from core.commands.registry.runtime import ArgRole, arg_indices_for_role

        # Defensive: a malformed ``trace add variable`` (no name) must not
        # blow up or invent an index.
        assert arg_indices_for_role("trace", ["add", "variable"], ArgRole.VAR_WRITE) == set()
        assert arg_indices_for_role("trace", ["add"], ArgRole.VAR_WRITE) == set()
        assert arg_indices_for_role("trace", [], ArgRole.VAR_WRITE) == set()


class TestOOAndSnitStructuralBodies:
    """Issue #250 — TclOO and snit method bodies are structural.

    ``oo::class`` (and the sister metaclasses ``oo::abstract``,
    ``oo::singleton``, ``oo::configurable``), ``oo::define``,
    ``snit::method`` / ``snit::typemethod`` and ``uri::register`` all
    take body arguments that run in their own definition / dispatch
    context — not in the caller's scope.  Marking them
    :class:`BodyKind.STRUCTURAL` keeps SSA from scanning their bodies as
    part of the enclosing block's data flow, mirroring ``proc`` /
    ``when`` / ``tcltest::test``.
    """

    def test_oo_class_create_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["create", "FOO", "body"]
        assert body_kind_for_command("oo::class", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("oo::class", args, ArgRole.BODY) == {2}

    def test_oo_class_new_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["new", "body"]
        assert body_kind_for_command("oo::class", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("oo::class", args, ArgRole.BODY) == {1}

    def test_oo_class_create_with_namespace_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["createWithNamespace", "FOO", "ns", "body"]
        assert body_kind_for_command("oo::class", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("oo::class", args, ArgRole.BODY) == {3}

    def test_sister_oo_metaclasses_match_oo_class(self):
        from core.commands.registry.runtime import BodyKind, body_kind_for_command

        # All four metaclasses share ``_oo_metaclass_arg_roles`` and must
        # share the structural-body classification — otherwise the same
        # ``cls create FOO { defScript }`` form would be analysed
        # differently depending on the metaclass.
        for cmd in ("oo::abstract", "oo::singleton", "oo::configurable"):
            assert body_kind_for_command(cmd, ["create", "FOO", "body"]) is BodyKind.STRUCTURAL, cmd

    def test_oo_define_method_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["Cls", "method", "foo", "args", "body"]
        assert body_kind_for_command("oo::define", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("oo::define", args, ArgRole.BODY) == {4}

    def test_oo_define_constructor_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["Cls", "constructor", "args", "body"]
        assert body_kind_for_command("oo::define", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("oo::define", args, ArgRole.BODY) == {3}

    def test_oo_define_destructor_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["Cls", "destructor", "body"]
        assert body_kind_for_command("oo::define", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("oo::define", args, ArgRole.BODY) == {2}

    def test_oo_define_bare_definition_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        # ``oo::define Cls { defScript }`` — the 2-arg form.
        args = ["Cls", "body"]
        assert body_kind_for_command("oo::define", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("oo::define", args, ArgRole.BODY) == {1}

    def test_snit_method_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["Type", "name", "arglist", "body"]
        assert body_kind_for_command("snit::method", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("snit::method", args, ArgRole.BODY) == {3}

    def test_snit_typemethod_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["Type", "name", "arglist", "body"]
        assert body_kind_for_command("snit::typemethod", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("snit::typemethod", args, ArgRole.BODY) == {3}

    def test_uri_register_handler_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["scheme", "script"]
        assert body_kind_for_command("uri::register", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("uri::register", args, ArgRole.BODY) == {1}

    def test_oo_objdefine_method_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["$obj", "method", "foo", "args", "body"]
        assert body_kind_for_command("oo::objdefine", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("oo::objdefine", args, ArgRole.BODY) == {4}

    def test_oo_objdefine_bare_definition_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        # ``oo::objdefine $obj { defScript }`` — the 2-arg form.
        args = ["$obj", "body"]
        assert body_kind_for_command("oo::objdefine", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("oo::objdefine", args, ArgRole.BODY) == {1}

    def test_snit_type_definition_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["Dog", "body"]
        assert body_kind_for_command("snit::type", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("snit::type", args, ArgRole.BODY) == {1}

    def test_snit_widget_definition_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["MyW", "body"]
        assert body_kind_for_command("snit::widget", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("snit::widget", args, ArgRole.BODY) == {1}

    def test_snit_widgetadaptor_definition_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["MyW", "body"]
        assert body_kind_for_command("snit::widgetadaptor", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("snit::widgetadaptor", args, ArgRole.BODY) == {1}

    def test_snit_compile_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["type", "name", "body"]
        assert body_kind_for_command("snit::compile", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("snit::compile", args, ArgRole.BODY) == {2}

    def test_snit_macro_body_is_structural(self):
        from core.commands.registry.runtime import (
            ArgRole,
            BodyKind,
            arg_indices_for_role,
            body_kind_for_command,
        )

        args = ["name", "arglist", "body"]
        assert body_kind_for_command("snit::macro", args) is BodyKind.STRUCTURAL
        assert arg_indices_for_role("snit::macro", args, ArgRole.BODY) == {2}


class TestCanonicalCommandHelper:
    """Issue #246 — ``to_canonical_command`` resolves aliases and qualifies.

    This is the lowering-time canonicalisation entry point.  Every IR
    construction site stamps ``canonical_command`` via this helper so
    downstream comparisons against literal strings (``stmt.canonical_command
    == "::unset"`` etc.) hit identically regardless of how the call was
    written in source.
    """

    def test_bare_name_qualifies_to_global(self):
        from core.common.alias import to_canonical_command

        assert to_canonical_command("unset") == "::unset"
        assert to_canonical_command("dict") == "::dict"
        assert to_canonical_command("set") == "::set"

    def test_already_qualified_name_normalises(self):
        from core.common.alias import to_canonical_command

        assert to_canonical_command("::unset") == "::unset"
        assert to_canonical_command("::ns::foo") == "::ns::foo"
        # Empty namespace components collapse: ``::ns::::foo`` → ``::ns::foo``.
        assert to_canonical_command("::ns::::foo") == "::ns::foo"

    def test_alias_resolves_to_target(self):
        from core.common.alias import to_canonical_command

        # interp alias {} = {} expr — calling ``=`` canonicalises to ``::expr``.
        aliases = {"::=": ("expr", ())}
        assert to_canonical_command("=", aliases=aliases) == "::expr"
        assert to_canonical_command("::=", aliases=aliases) == "::expr"

    def test_alias_chain_terminates_at_self_loop(self):
        from core.common.alias import to_canonical_command

        # Self-referential alias: must not loop forever.
        aliases = {"::loop": ("loop", ())}
        assert to_canonical_command("loop", aliases=aliases) == "::loop"

    def test_alias_chain_terminates_on_cycle(self):
        from core.common.alias import to_canonical_command

        # ``a -> b -> a`` cycle — the helper breaks at the first repeat.
        aliases = {"::a": ("b", ()), "::b": ("a", ())}
        # Either ``::a`` or ``::b`` is acceptable as a terminating point;
        # what matters is the call returns rather than looping.
        result = to_canonical_command("a", aliases=aliases)
        assert result in ("::a", "::b")

    def test_namespace_aware_alias_lookup(self):
        from core.common.alias import to_canonical_command

        # An alias scoped to ``::ns`` is reachable from a bare call inside
        # that namespace but not from the global scope.
        aliases = {"::ns::myunset": ("unset", ())}
        assert to_canonical_command("myunset", namespace="::ns", aliases=aliases) == "::unset"
        # From global scope the alias is invisible — bare ``myunset`` falls
        # through to its own global form.
        assert to_canonical_command("myunset", aliases=aliases) == "::myunset"


class TestIRCallCanonicalCommand:
    """Issue #246 — every IRCall / IRBarrier produced by lowering carries
    a canonical command name regardless of source spelling.

    This is the contract the systematic audit (item 10) relies on: any
    pass that switches on a literal command name must be able to compare
    against ``stmt.canonical_command`` and have qualified forms, aliases,
    and namespace-scoped calls all hit the same branch.
    """

    @staticmethod
    def _walk(stmts):
        from core.compiler.ir import IRBarrier, IRBlock, IRCall

        for s in stmts:
            if isinstance(s, (IRCall, IRBarrier)):
                yield s
            if isinstance(s, IRBlock):
                yield from TestIRCallCanonicalCommand._walk(s.body.statements)

    def _lower(self, src):
        from core.compiler.lowering import _Lowerer

        return _Lowerer().lower(src)

    def test_bare_command_canonicalises_to_global(self):
        m = self._lower("unset x")
        calls = list(self._walk(m.top_level.statements))
        # ``unset`` produces an IRCall with canonical "::unset".
        assert any(c.canonical_command == "::unset" for c in calls)

    def test_qualified_command_canonicalises_identically(self):
        m_bare = self._lower("unset x")
        m_qual = self._lower("::unset x")
        bare = next(c for c in self._walk(m_bare.top_level.statements) if "unset" in c.command)
        qual = next(c for c in self._walk(m_qual.top_level.statements) if "unset" in c.command)
        assert bare.canonical_command == qual.canonical_command == "::unset"

    def test_namespace_eval_body_canonicalises_to_global(self):
        # Inside ``namespace eval ns``, a bare ``unset z`` still resolves
        # to ``::unset`` (the global builtin) — namespace shadowing of
        # builtin commands is rare and not statically tracked here.
        m = self._lower("namespace eval ns { unset z }")
        calls = list(self._walk(m.top_level.statements))
        unsets = [c for c in calls if "unset" in c.command]
        assert unsets and all(c.canonical_command == "::unset" for c in unsets)

    def test_interp_alias_resolves_at_lowering(self):
        src = """
        interp alias {} myunset {} unset
        myunset w
        ::myunset q
        """
        m = self._lower(src)
        calls = list(self._walk(m.top_level.statements))
        # Both ``myunset w`` and ``::myunset q`` should canonicalise to ``::unset``.
        unsets = [c for c in calls if c.canonical_command == "::unset"]
        assert len(unsets) >= 2

    def test_diagnostic_targets_canonical_form_uniformly(self):
        # W213 (read-before-set on possibly-undefined unset target) is one
        # of the diagnostics that switches on ``stmt.canonical_command ==
        # "::unset"``.  An aliased ``myunset`` must trigger it identically
        # to a bare or qualified ``unset``.  Lowering produces canonical
        # ``::unset`` for both — this test exercises the IR-level contract.
        # (Actual diagnostic fixture coverage lives in tests/test_analyser.py.)
        src = """
        interp alias {} myunset {} unset
        myunset z
        """
        m = self._lower(src)
        calls = list(self._walk(m.top_level.statements))
        aliased_unset = next((c for c in calls if c.canonical_command == "::unset"), None)
        assert aliased_unset is not None

    def test_cfg_synthetic_calls_carry_canonical(self):
        # The CFG lowering emits synthetic IRCalls for catch / try / foreach /
        # lmap / dict iteration when the loop is collapsed for codegen.
        # These synthetic constructs must also carry canonical_command so
        # downstream passes that match on it (e.g. SSA def-use) treat them
        # uniformly with lowering-produced calls.
        from core.compiler.cfg import build_cfg

        m = self._lower("foreach x {1 2 3} { puts $x }")
        cfg_module = build_cfg(m)
        # ``__top__`` holds the top-level CFG — drill down into its blocks.
        top_cfg = cfg_module.top_level
        canonical_seen = {
            stmt.canonical_command
            for block in top_cfg.blocks.values()
            for stmt in block.statements
            if hasattr(stmt, "canonical_command") and stmt.canonical_command
        }
        # The foreach var-def synthetic IRCall must show up canonical.
        assert "::foreach" in canonical_seen, (
            f"Expected synthetic IRCall with canonical_command='::foreach', saw {canonical_seen}"
        )


class TestCanonicalisationAuditMarkers:
    """Issue #246 — files with literal command-name comparisons must
    declare they have been audited and migrated to canonical form.

    The audit marker is a top-level comment ``# canonicalisation:
    audited #246``.  Adding it asserts that every ``stmt.command ==
    "..."`` and ``stmt.command in (...)`` style check in the file
    either uses the canonical form (``::cmd``) or is a documented
    bucket-(iii) pass-specific identity check.
    """

    AUDITED = (
        "core/compiler/cfg.py",
        "core/compiler/core_analyses.py",
        "core/compiler/compiler_checks.py",
        "core/compiler/inlining/inline_pass.py",
        "core/compiler/irules_flow.py",
        "core/compiler/inline_uplevel.py",
        "core/compiler/memory_ssa.py",
        "core/compiler/taint/_uri_split.py",
        "core/compiler/taint/_sinks.py",
        "core/compiler/lowering.py",
        "core/compiler/lowering_hooks/_var.py",
        "core/compiler/lowering_hooks/_control.py",
        "core/compiler/source_inliner.py",
        "core/compiler/optimiser/_pattern_recognition.py",
        "core/compiler/passes/specialise_factories.py",
        "core/compiler/interprocedural.py",
        "core/compiler/codegen/_emitter.py",
        "core/compiler/codegen/_control_flow.py",
        "core/compiler/codegen/wasm_link.py",
        "core/compiler/codegen/wasm/_scan.py",
        "core/compiler/codegen/wasm/_emitter/_statements.py",
        "core/compiler/codegen/wasm/_emitter/_optimisation.py",
        "core/compiler/codegen/wasm/_emitter/_control_flow.py",
        "core/compiler/codegen/wasm/_emitter/_commands.py",
        "core/compiler/var_escape/_propagation.py",
        "core/compiler/var_escape/_cfg_propagation.py",
        "core/diagram/extract.py",
        "core/analysis/_analyser/_diag_commands.py",
        "core/analysis/_analyser/_diag_var_command.py",
        "core/analysis/_analyser/_diag_var_lifecycle.py",
        "core/analysis/_analyser/_diag_racy.py",
    )

    MARKER = "# canonicalisation: audited #246"

    def test_audited_files_carry_marker(self):
        from pathlib import Path

        repo_root = Path(__file__).resolve().parent.parent
        missing = [
            path for path in self.AUDITED if self.MARKER not in (repo_root / path).read_text()
        ]
        assert missing == [], (
            f"Files migrated to canonical command matching must declare the audit marker "
            f"``{self.MARKER}``; missing in: {missing}"
        )

    def test_audited_files_have_no_unmarked_literal_command_checks(self):
        """Reject ``stmt.command == "name"`` / ``stmt.command in ("a", "b")``
        in audited files — they must use ``stmt.canonical_command == "::name"``
        or carry an explicit bucket-(iii) marker on the line.
        """
        import re
        from pathlib import Path

        repo_root = Path(__file__).resolve().parent.parent
        # Pattern: ``stmt.command == "x"`` or ``cmd.command in ("x", ...)``
        # (allows ``ir_stmt.command``, ``s.command``, etc.).
        # Match ``X.command == "..."`` / ``X.command != "..."`` /
        # ``X.command in (...)``.  The ``\.`` anchor avoids flagging a
        # local parameter named ``command`` in unrelated helpers; the
        # ``(?<!canonical_)`` negative-lookbehind keeps the new
        # ``stmt.canonical_command`` form whitelisted.
        pattern = re.compile(r'(?<!canonical_)\.command\s*(==|!=|in)\s*[\("]')
        bucket_iii = "# canonicalisation: bucket-iii"
        # The bucket-(iii) marker may appear on the violation line OR
        # on any of the four lines immediately preceding it — the
        # auto-formatter sometimes wraps long expressions across
        # multiple lines, leaving the marker on a sibling line.
        marker_lookback = 4
        violations: list[str] = []
        for path in self.AUDITED:
            full = repo_root / path
            lines = full.read_text().splitlines()
            for lineno, line in enumerate(lines, 1):
                if not pattern.search(line):
                    continue
                # Skip docstrings and comment lines that just mention the pattern.
                stripped = line.lstrip()
                if stripped.startswith("#") or stripped.startswith('"') or stripped.startswith("'"):
                    continue
                # Marker check: same line, or any of the preceding
                # ``marker_lookback`` lines.
                window_start = max(0, lineno - 1 - marker_lookback)
                window = lines[window_start:lineno]
                if any(bucket_iii in w for w in window):
                    continue
                violations.append(f"{path}:{lineno}: {line.strip()}")
        assert violations == [], (
            "Audited files contain unmarked literal command-name checks. Either migrate to "
            '``stmt.canonical_command == "::name"`` or add the bucket-(iii) marker '
            "``# canonicalisation: bucket-iii — <reason>`` on the same line.\n"
            + "\n".join(violations)
        )


class TestVariableNameAndDisplayHelpers:
    """Issue #246 — ``to_canonical_var`` / ``from_canonical`` /
    ``is_canonical_command`` round out the helper surface for variable
    naming and source-spelling rendering.
    """

    def test_to_canonical_var_strips_sigils(self):
        from core.common.naming import to_canonical_var

        assert to_canonical_var("x") == "x"
        assert to_canonical_var("$x") == "x"
        assert to_canonical_var("${x}") == "x"
        assert to_canonical_var("arr(key)") == "arr"

    def test_to_canonical_var_preserves_qualified_form(self):
        from core.common.naming import to_canonical_var

        # Bare locals stay bare; qualified globals stay qualified.
        # The bare/qualified distinction is semantically meaningful
        # (local vs global) and is preserved.
        assert to_canonical_var("::g") == "::g"
        assert to_canonical_var("::ns::v") == "::ns::v"

    def test_from_canonical_strips_global_prefix(self):
        from core.common.naming import from_canonical

        assert from_canonical("::set") == "set"
        assert from_canonical("::unset") == "unset"

    def test_from_canonical_preserves_nested_namespace(self):
        from core.common.naming import from_canonical

        # ``::tcl::dict::for`` from global scope keeps the qualifier
        # so the reader sees the cross-namespace reach.
        assert from_canonical("::tcl::dict::for") == "::tcl::dict::for"

    def test_from_canonical_rebases_to_display_namespace(self):
        from core.common.naming import from_canonical

        assert from_canonical("::ns::foo", display_namespace="::ns") == "foo"
        # Sibling namespace: qualified rendering preserved.
        assert from_canonical("::other::foo", display_namespace="::ns") == "::other::foo"

    def test_is_canonical_command_accepts_qualified(self):
        from core.common.naming import is_canonical_command

        assert is_canonical_command("::set") is True
        assert is_canonical_command("::HTTP::respond") is True
        assert is_canonical_command("::tcl::dict::for") is True

    def test_is_canonical_command_rejects_bare(self):
        from core.common.naming import is_canonical_command

        assert is_canonical_command("set") is False
        assert is_canonical_command("dict") is False

    def test_is_canonical_command_accepts_synthetic(self):
        from core.common.naming import is_canonical_command

        # CFG synthetic nodes have no user-source command.
        assert is_canonical_command("") is True
        assert is_canonical_command("<cond>") is True
        assert is_canonical_command("<empty_clause>") is True
        assert is_canonical_command("<upvar-invalidate>") is True


class TestRenameAndNamespaceImportCanonicalisation:
    """Issue #246 — per-call-site canonicalisation follows ``rename``
    and ``namespace import`` directives captured at lowering time, so
    the IR's ``canonical_command`` reflects the underlying command
    even when the user wrote a redirected or imported spelling.
    """

    @staticmethod
    def _walk(stmts):
        from core.compiler.ir import IRBarrier, IRBlock, IRCall

        for s in stmts:
            if isinstance(s, (IRCall, IRBarrier)):
                yield s
            if isinstance(s, IRBlock):
                yield from TestRenameAndNamespaceImportCanonicalisation._walk(s.body.statements)

    def _lower(self, src):
        from core.compiler.lowering import _Lowerer

        return _Lowerer().lower(src)

    def test_rename_redirects_canonical_to_original(self):
        # ``rename unset myunset`` then ``myunset x`` — the canonical
        # form follows the rename to the underlying builtin.
        src = "rename unset myunset\nmyunset x"
        m = self._lower(src)
        myunset_call = next(
            (c for c in self._walk(m.top_level.statements) if c.command == "myunset"),
            None,
        )
        assert myunset_call is not None
        assert myunset_call.canonical_command == "::unset"

    def test_rename_then_alias_chains_to_builtin(self):
        # A double redirect: rename then alias.  Lowering's alias
        # resolution rewrites ``mu`` to ``myunset`` at IR construction
        # (``IRCall.command`` becomes ``myunset``); the canonicaliser
        # then follows the captured ``rename myunset → unset`` to
        # produce ``::unset`` as the canonical form.
        src = """
        rename unset myunset
        interp alias {} mu {} myunset
        mu x
        """
        m = self._lower(src)
        unset_calls = [
            c for c in self._walk(m.top_level.statements) if c.canonical_command == "::unset"
        ]
        assert unset_calls, (
            "expected at least one IRCall canonicalising to '::unset' from the rename + alias chain"
        )

    def test_namespace_import_glob_resolves_to_source(self):
        # ``namespace import ::tcltest::*`` then a bare ``test`` call
        # inside the importing namespace canonicalises to the source
        # name ``::tcltest::test``.
        src = """
        namespace eval app {
          namespace import ::tcltest::*
          test t1 desc -body {expr 1}
        }
        """
        m = self._lower(src)
        test_call = next(
            (
                c
                for c in self._walk(m.top_level.statements)
                if "test" in c.command and c.command != "namespace"
            ),
            None,
        )
        assert test_call is not None
        assert test_call.canonical_command == "::tcltest::test"

    def test_namespace_import_does_not_shadow_builtin(self):
        # ``namespace import ::tcltest::*`` should NOT rewrite a plain
        # ``namespace`` call to ``::tcltest::namespace`` — Tcl's import
        # shadows builtins at runtime, but the static canonicalisation
        # preserves the global-builtin binding so analysis still
        # recognises ``namespace eval`` patterns under the import.
        src = """
        namespace eval app {
          namespace import ::tcltest::*
          namespace eval inner { set y 1 }
        }
        """
        m = self._lower(src)
        # Walk top-level + the IRBlock(ns=::app) — find the inner
        # namespace-eval call (which lowers to an IRBlock at the app
        # level whose source IRCall is ``namespace`` with canonical
        # ``::namespace``).
        ns_call = next(
            (
                c
                for c in self._walk(m.top_level.statements)
                if c.command == "namespace" and c.args and c.args[0] == "import"
            ),
            None,
        )
        # The ``namespace import`` call itself canonicalises to
        # ``::namespace`` (the builtin), not ``::tcltest::namespace``
        # — the import-shortcut guard we want to assert is that the
        # builtin lookup wins.
        assert ns_call is not None
        assert ns_call.canonical_command == "::namespace"

    def test_namespace_import_dead_code_does_not_register(self):
        # ``namespace import`` inside ``if {0} { … }`` is dead code;
        # the canonicaliser must not consult its pattern.
        src = """
        namespace eval app {
          if {0} {
            namespace import ::tcltest::*
          }
          test t1 desc -body {expr 1}
        }
        """
        m = self._lower(src)
        test_call = next(
            (
                c
                for c in self._walk(m.top_level.statements)
                if "test" in c.command and c.command != "namespace"
            ),
            None,
        )
        # Without the import being captured (dead-code gate), bare
        # ``test`` falls back to the global ``::test`` form.
        assert test_call is not None
        # The static-export over-approximation still maps a bare-name
        # call to ``::tcltest::test`` for body-kind queries (see the
        # cfd0e12 first slice), but the canonical form here is just
        # the global form.  Either is acceptable; what we care about
        # is that the dead ``namespace import`` did NOT contribute.
        assert test_call.canonical_command == "::test"
