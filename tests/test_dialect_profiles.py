"""Tests for configurable command-signature dialect profiles."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.commands.registry.runtime import (
    SIGNATURES,
    ArgRole,
    CommandSig,
    SubcommandSig,
    active_signature_profile,
    arg_indices_for_role,
    configure_signatures,
)
from lsp.features.completion import get_completions


def _command_sig(name: str) -> CommandSig:
    sig = SIGNATURES[name]
    assert isinstance(sig, CommandSig)
    return sig


def _subcommand_sig(name: str) -> SubcommandSig:
    sig = SIGNATURES[name]
    assert isinstance(sig, SubcommandSig)
    return sig


class TestDialectProfiles:
    def test_tcl84_removes_newer_commands(self):
        configure_signatures(dialect="tcl8.4")
        assert "dict" not in SIGNATURES
        assert "try" not in SIGNATURES
        assert "tailcall" not in SIGNATURES

    def test_tcl85_keeps_dict_but_not_try(self):
        configure_signatures(dialect="tcl8.5")
        assert "dict" in SIGNATURES
        assert "try" not in SIGNATURES
        assert active_signature_profile()["dialect"] == "tcl8.5"

    def test_unknown_dialect_is_ignored(self):
        configure_signatures(dialect="tcl8.4")
        changed = configure_signatures(dialect="8.4")
        assert changed is False
        assert active_signature_profile()["dialect"] == "tcl8.4"

    def test_f5_profile_adds_when_and_http_commands(self):
        configure_signatures(dialect="f5-irules")
        assert "when" in SIGNATURES
        assert "HTTP::header" in SIGNATURES
        # Pulled from BIG-IP iRules seed data; guards full catalog import.
        assert "AAA::acct_result" in SIGNATURES

    def test_when_marks_body_argument(self):
        configure_signatures(dialect="f5-irules")
        indices = arg_indices_for_role("when", ["HTTP_REQUEST", "{ puts ok }"], ArgRole.BODY)
        assert indices == {1}

    def test_when_marks_last_argument_body_in_extended_form(self):
        configure_signatures(dialect="f5-irules")
        indices = arg_indices_for_role(
            "when",
            ["CLIENT_ACCEPTED", "priority", "500", "{ puts ok }"],
            ArgRole.BODY,
        )
        assert indices == {3}

    def test_tcl86_includes_tcloo_commands(self):
        configure_signatures(dialect="tcl8.6")
        assert "oo::class" in SIGNATURES
        assert "oo::define" in SIGNATURES

    def test_tcl85_excludes_tcloo_commands(self):
        configure_signatures(dialect="tcl8.5")
        assert "oo::class" not in SIGNATURES
        assert "oo::define" not in SIGNATURES

    def test_oo_class_create_marks_definition_body(self):
        configure_signatures(dialect="tcl8.6")
        indices = arg_indices_for_role(
            "oo::class",
            ["create", "Dog", "{ method bark {} { puts ok } }"],
            ArgRole.BODY,
        )
        assert indices == {2}

    def test_oo_define_method_marks_method_body(self):
        configure_signatures(dialect="tcl8.6")
        indices = arg_indices_for_role(
            "oo::define",
            ["Dog", "method", "bark", "{}", "{ puts ok }"],
            ArgRole.BODY,
        )
        assert indices == {4}

    def test_oo_define_script_form_marks_body(self):
        configure_signatures(dialect="tcl8.6")
        indices = arg_indices_for_role(
            "oo::define",
            ["Dog", "{ method bark {} { puts ok } }"],
            ArgRole.BODY,
        )
        assert indices == {1}

    def test_f5_irules_target_family_signatures_are_concrete(self):
        configure_signatures(dialect="f5-irules")
        # Curated commands have concrete validation.
        sig = _subcommand_sig("HTTP::header")
        assert "value" in sig.subcommands
        assert "insert" in sig.subcommands
        assert _command_sig("when").arity.max == 6
        assert _command_sig("pool").arity.min == 1
        assert _command_sig("node").arity.min == 1
        assert _command_sig("HTTP::respond").arity.min == 1
        # Generated commands are present with baseline validation.
        assert "SSL::cert" in SIGNATURES
        assert "CRYPTO::keygen" in SIGNATURES
        assert "TCP::collect" in SIGNATURES
        assert "UDP::payload" in SIGNATURES
        assert "HTTP::path" in SIGNATURES
        assert "TCP::option" in SIGNATURES
        assert "UDP::respond" in SIGNATURES

    def test_f5_irules_generated_commands_are_present(self):
        configure_signatures(dialect="f5-irules")
        assert "class" in SIGNATURES
        assert "table" in SIGNATURES

    def test_f5_irules_includes_deprecated_and_disabled_commands(self):
        configure_signatures(dialect="f5-irules")
        assert "PROFILE::antifraud" in SIGNATURES
        assert "PROFILE::avr" in SIGNATURES
        assert "PROFILE::exchange" in SIGNATURES
        assert "PROFILE::list" in SIGNATURES
        assert "PROFILE::tftp" in SIGNATURES
        assert "PROFILE::vdi" in SIGNATURES
        assert "IP::ingress_rate_limit" in SIGNATURES
        assert "PSC::aaa_reporting_interval" in SIGNATURES
        assert "fasthash" in SIGNATURES

    def test_f5_irules_keeps_base_proc_signature(self):
        configure_signatures(dialect="f5-irules")
        assert _command_sig("proc").arity.min == 3
        assert _command_sig("proc").arity.max == 3
        indices = arg_indices_for_role("proc", ["helper", "x", "{ return $x }"], ArgRole.BODY)
        assert indices == {2}

    def test_f5_irules_disabled_commands_emit_warnings(self):
        configure_signatures(dialect="f5-irules")
        result = analyse("open /tmp/x\ntime { puts ok }")
        disabled_warnings = [d for d in result.diagnostics if d.code == "W002"]
        assert len(disabled_warnings) == 2
        assert any("'open' is disabled" in d.message for d in disabled_warnings)
        assert any("'time' is disabled" in d.message for d in disabled_warnings)

    def test_non_f5_profile_does_not_warn_on_same_commands(self):
        configure_signatures(dialect="tcl8.6")
        result = analyse("open /tmp/x\ntime { puts ok }")
        assert all(d.code != "W002" for d in result.diagnostics)

    def test_completion_hides_f5_irules_disabled_commands(self):
        configure_signatures(dialect="f5-irules")
        labels = {item.label for item in get_completions("", 0, 0)}
        assert "open" not in labels
        assert "exec" not in labels
        assert "namespace" not in labels

    def test_f5_irules_profile_is_large_catalog(self):
        configure_signatures(dialect="f5-irules")
        # Core Tcl signatures + iRules command corpus from BIG-IP docs.
        assert len(SIGNATURES) > 1000

    def test_tcllib_commands_in_signatures(self):
        # Tcllib commands are always present in SIGNATURES (namespaced,
        # no collision with core Tcl).  Per-document filtering happens
        # at the feature layer via ``package require``.
        configure_signatures(dialect="tcl8.6")
        assert "json::json2dict" in SIGNATURES
        assert "base64::encode" in SIGNATURES

    def test_f5_iapps_profile_adds_iapp_utility_commands(self):
        configure_signatures(dialect="f5-iapps")
        assert "iapp::template" in SIGNATURES
        assert "iapp::conf" in SIGNATURES
        # f5-iapps is separate from the iRules catalog.
        assert "AAA::acct_result" not in SIGNATURES

    def test_completion_reflects_active_profile(self):
        configure_signatures(dialect="f5-iapps")
        labels = {item.label for item in get_completions("", 0, 0)}
        assert "iapp::template" in labels

    def test_tcllib_completion_requires_package_require(self):
        source = "package require json\n"
        labels = {item.label for item in get_completions(source, 1, 0)}
        assert "json::json2dict" in labels

    def test_tcllib_completion_absent_without_package_require(self):
        labels = {item.label for item in get_completions("", 0, 0)}
        assert "json::json2dict" not in labels
