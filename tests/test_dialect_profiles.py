"""Tests for configurable command-signature dialect profiles."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.registry.runtime import (
    SIGNATURES,
    ArgRole,
    CommandSig,
    SubcommandSig,
    active_signature_profile,
    arg_indices_for_role,
    configure_signatures,
)
from core.analysis import analyse
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

    def test_w002_suppressed_when_user_proc_shadows_disallowed_command(self):
        # tcllib's installer.tcl defines its own ``::log`` proc, which
        # shadows the iRules-only ``log`` built-in. W002 must not fire
        # on calls to ``log`` when the user has defined it before the
        # call site at an unconditional top-level position.
        configure_signatures(dialect="tcl8.6")
        src = 'proc ::log {text} { puts $text }\nlog "hello"\n'
        result = analyse(src)
        assert all(d.code != "W002" for d in result.diagnostics)

    def test_w002_fires_without_user_proc(self):
        configure_signatures(dialect="tcl8.6")
        result = analyse('log "hello"\n')
        w002 = [d for d in result.diagnostics if d.code == "W002"]
        assert len(w002) == 1
        assert "'log' is disabled" in w002[0].message

    def test_w002_suppressed_for_namespaced_user_proc(self):
        # ``namespace eval`` runs unconditionally, so a proc it defines
        # counts as a shadowing definition for W002 purposes.
        configure_signatures(dialect="tcl8.6")
        src = 'namespace eval ::ns { proc log {text} { puts $text } }\n::ns::log "hi"\n'
        result = analyse(src)
        assert all(d.code != "W002" for d in result.diagnostics)

    def test_w002_fires_when_call_precedes_user_proc(self):
        # Command resolution is order-dependent at runtime: a call that
        # executes before ``proc ::log`` runs would dispatch to the
        # (disallowed) built-in, so W002 must still fire.
        configure_signatures(dialect="tcl8.6")
        src = 'log "hello"\nproc ::log {text} { puts $text }\n'
        result = analyse(src)
        w002 = [d for d in result.diagnostics if d.code == "W002"]
        assert len(w002) == 1
        assert "'log' is disabled" in w002[0].message

    def test_w002_fires_when_user_proc_is_defined_conditionally(self):
        # A proc defined inside an ``if`` body is not guaranteed to
        # exist at an arbitrary call site, so W002 still fires.
        configure_signatures(dialect="tcl8.6")
        src = 'if {1} { proc ::log {text} { puts $text } }\nlog "hello"\n'
        result = analyse(src)
        w002 = [d for d in result.diagnostics if d.code == "W002"]
        assert len(w002) == 1
        assert "'log' is disabled" in w002[0].message

    def test_w002_skipped_for_variable_as_command(self):
        # Issue #233: ``$table item style ...`` is variable-as-command
        # (a Tk widget handle, TclOO object, etc.). The literal name
        # ``table`` happens to be an iRules built-in (DISALLOWED in
        # plain Tcl), but the runtime command is whatever string the
        # variable holds. W002 must not fire on such substitution sites.
        configure_signatures(dialect="tcl8.6")
        src = (
            "proc compareRows {table item col} {\n"
            "    set s [$table item style set $item $col]\n"
            "    return $s\n"
            "}\n"
        )
        result = analyse(src)
        assert all(d.code != "W002" for d in result.diagnostics)

    def test_w002_skipped_for_command_substitution_as_command(self):
        # ``[lookup] arg`` is command-substitution-as-command. The
        # actual dispatched command is the substitution's runtime
        # *return value*, not the inner command name. Use a disabled
        # inner name (``open`` under f5-irules) to verify that W002
        # only fires for the inner literal call, never for the outer
        # ``[`` site — even though the outer site's first token is a
        # CMD token whose text starts with ``open``.
        configure_signatures(dialect="f5-irules")
        src = "[open /tmp/x] arg\n"
        result = analyse(src)
        w002 = [d for d in result.diagnostics if d.code == "W002"]
        # Exactly one W002, on the *inner* literal ``open`` (offset 1),
        # never on the outer ``[`` site (offset 0).
        assert len(w002) == 1
        assert w002[0].range.start.offset == 1
        assert "'open' is disabled" in w002[0].message

    def test_w002_fires_for_lattice_resolved_disabled_command(self):
        # When the lattice statically resolves ``$cmd`` to a literal
        # disabled command name, W002 still fires — variable-as-command
        # is suppressed, but only when the dispatched command cannot
        # be statically pinned.
        configure_signatures(dialect="f5-irules")
        src = "set cmd open\n$cmd /tmp/x\n"
        result = analyse(src)
        w002 = [d for d in result.diagnostics if d.code == "W002"]
        assert len(w002) == 1
        assert "'open' is disabled" in w002[0].message

    def test_w002_skipped_for_composite_substitution_command_word(self):
        # ``${cmd}x`` dispatches to ``<value>x`` at runtime, not the
        # variable's value. Even when the lattice resolves ``$cmd`` to
        # a disabled command name like ``open``, W002 must not fire on
        # the composite word because the actual concatenated command
        # (``openx``) is unknown.
        configure_signatures(dialect="f5-irules")
        src = "set cmd open\n${cmd}x /tmp/x\n"
        result = analyse(src)
        assert all(d.code != "W002" for d in result.diagnostics)

    def test_w307_not_suppressed_for_composite_var_command_word(self):
        # The CONSTSET-based W307 suppression must also gate on
        # single-token-ness: ``${cmd}x`` is the concatenation
        # ``<value>x``, not the variable's value, so the lattice
        # resolution of ``$cmd`` to a known command name does not
        # tell us what the actual dispatched command is.  W307 must
        # still fire on the composite outer word.
        configure_signatures(dialect="tcl8.6")
        src = "set cmd puts\n${cmd}x hello\n"
        result = analyse(src)
        assert any(d.code == "W307" for d in result.diagnostics)

    def test_w308_not_emitted_for_composite_cmd_substitution_word(self):
        # ``[Dog new]x extra`` dispatches to ``<object_handle>x``,
        # not ``[Dog new] extra`` — so even though ``Dog new`` returns
        # a Dog instance, the method-validation post-pass must not
        # treat ``extra`` as a method call on Dog.
        configure_signatures(dialect="tcl8.6")
        src = "oo::class create Dog {\n    method bark {} { return woof }\n}\n[Dog new]x extra\n"
        result = analyse(src)
        assert all(d.code != "W308" for d in result.diagnostics)

    def test_w002_fires_for_fully_qualified_disallowed_command(self):
        # The command registry is keyed without a leading ``::``; W002
        # must strip it so ``::open`` under the iRules dialect is
        # recognised as the disallowed ``open`` built-in.
        configure_signatures(dialect="f5-irules")
        result = analyse("::open /tmp/x\n")
        w002 = [d for d in result.diagnostics if d.code == "W002"]
        assert len(w002) == 1
        assert "'::open' is disabled" in w002[0].message

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

    # Expect dialect

    def test_expect_profile_adds_expect_commands(self):
        configure_signatures(dialect="expect")
        assert "spawn" in SIGNATURES
        assert "expect" in SIGNATURES
        assert "send" in SIGNATURES
        assert "interact" in SIGNATURES
        assert "log_user" in SIGNATURES

    def test_expect_profile_includes_base_tcl(self):
        configure_signatures(dialect="expect")
        assert "set" in SIGNATURES
        assert "proc" in SIGNATURES
        assert "if" in SIGNATURES

    def test_expect_profile_does_not_include_irules(self):
        configure_signatures(dialect="expect")
        assert "when" not in SIGNATURES
        assert "HTTP::header" not in SIGNATURES

    def test_expect_body_in_expect_command(self):
        configure_signatures(dialect="expect")
        indices = arg_indices_for_role(
            "expect",
            ["-re", "password:", "{ send secret\\r }"],
            ArgRole.BODY,
        )
        assert indices == {2}

    def test_completion_reflects_expect_profile(self):
        configure_signatures(dialect="expect")
        labels = {item.label for item in get_completions("", 0, 0)}
        assert "spawn" in labels
        assert "expect" in labels
