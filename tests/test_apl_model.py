"""Tests for the APL structured model and cross-file integration."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.apl_model import (
    apl_name_to_tcl_var,
    parse_apl,
    resolve_apl_includes,
    tcl_var_to_apl_name,
)
from core.bigip.iapp_diagnostics import (
    validate_iapp_implementation,
    validate_iapp_presentation,
)
from core.bigip.iapp_vars import extract_iapp_var_refs


class TestAplModel:
    """Tests for APL structured model parsing."""

    def test_parse_section_with_fields(self):
        source = (
            "section basic {\n"
            '    string addr default "0.0.0.0" required\n'
            '    string port default "443"\n'
            "}\n"
        )
        model = parse_apl(source)
        assert "basic" in model.sections
        sec = model.sections["basic"]
        assert "addr" in sec.fields
        assert "port" in sec.fields
        assert sec.fields["addr"].qualified_name == "basic.addr"
        assert sec.fields["addr"].is_required is True
        assert sec.fields["port"].is_required is False

    def test_parse_table_with_columns(self):
        source = (
            "table members {\n"
            '    string addr required validator "IpAddress"\n'
            '    string port default "80"\n'
            "}\n"
        )
        model = parse_apl(source)
        assert "members" in model.tables
        tbl = model.tables["members"]
        assert "addr" in tbl.columns
        assert "port" in tbl.columns
        assert tbl.columns["addr"].qualified_name == "members.addr"

    def test_parse_defines(self):
        source = "define choice yesno_choice {\n}\n"
        model = parse_apl(source)
        assert "yesno_choice" in model.defines
        assert model.defines["yesno_choice"] == "choice"

    def test_parse_includes(self):
        source = '#include "f5.apl_common"\n#include "utils.apl"\n'
        model = parse_apl(source)
        assert len(model.includes) == 2
        assert model.includes[0].path == "f5.apl_common"
        assert model.includes[1].path == "utils.apl"

    def test_all_fields_flattened(self):
        source = (
            "section basic {\n"
            "    string addr\n"
            "    string port\n"
            "}\n"
            "section ssl {\n"
            "    string cert_name\n"
            "}\n"
            "table members {\n"
            "    string ip\n"
            "}\n"
        )
        model = parse_apl(source)
        assert "basic.addr" in model.all_fields
        assert "basic.port" in model.all_fields
        assert "ssl.cert_name" in model.all_fields
        assert "members.ip" in model.all_fields

    def test_tcl_variable_names(self):
        source = "section basic {\n    string addr\n    string port\n}\n"
        model = parse_apl(source)
        tcl_vars = model.tcl_variable_names()
        assert "::basic__addr" in tcl_vars
        assert "::basic__port" in tcl_vars

    def test_field_types(self):
        source = (
            "section fields {\n    string s1\n    choice c1\n    password pw1\n    yesno yn1\n}\n"
        )
        model = parse_apl(source)
        sec = model.sections["fields"]
        assert sec.fields["s1"].field_type == "string"
        assert sec.fields["c1"].field_type == "choice"
        assert sec.fields["pw1"].field_type == "password"
        assert sec.fields["yn1"].field_type == "yesno"


class TestAplIncludeResolution:
    """Tests for #include file resolution."""

    def test_resolve_include(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            # Write included file
            inc_path = os.path.join(tmpdir, "common.apl")
            Path(inc_path).write_text("section shared {\n    string common_field\n}\n")
            # Write main file
            source = '#include "common.apl"\nsection basic {\n    string addr\n}\n'
            model = resolve_apl_includes(source, tmpdir)
            assert "basic" in model.sections
            assert "shared" in model.sections
            assert "shared.common_field" in model.all_fields

    def test_unresolved_include(self):
        source = '#include "nonexistent.apl"\n'
        model = resolve_apl_includes(source, "/tmp/nonexistent_dir_xyz")
        assert len(model.includes) == 1
        assert model.includes[0].resolved is False

    def test_circular_include_guarded(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            # File A includes B, B includes A
            Path(os.path.join(tmpdir, "a.apl")).write_text(
                '#include "b.apl"\nsection sa { string fa }\n'
            )
            Path(os.path.join(tmpdir, "b.apl")).write_text(
                '#include "a.apl"\nsection sb { string fb }\n'
            )
            with open(os.path.join(tmpdir, "a.apl")) as f:
                source = f.read()
            model = resolve_apl_includes(source, tmpdir)
            # Should not infinite loop; both sections should be present
            assert "sa" in model.sections
            assert "sb" in model.sections


class TestTclVarMapping:
    """Tests for Tcl ↔ APL variable name conversion."""

    def test_tcl_var_to_apl_name(self):
        assert tcl_var_to_apl_name("::basic__addr") == "basic.addr"
        assert tcl_var_to_apl_name("basic__addr") == "basic.addr"
        assert tcl_var_to_apl_name("::pool__members__addr") == "pool.members.addr"

    def test_tcl_var_no_double_underscore(self):
        assert tcl_var_to_apl_name("::simple_var") is None
        assert tcl_var_to_apl_name("noprefix") is None

    def test_apl_name_to_tcl_var(self):
        assert apl_name_to_tcl_var("basic.addr") == "::basic__addr"
        assert apl_name_to_tcl_var("pool.members.addr") == "::pool__members__addr"


class TestIappVarExtraction:
    """Tests for iApp variable reference extraction from Tcl."""

    def test_extract_simple_var(self):
        source = "set addr $::basic__addr\nset port $::basic__port\n"
        refs = extract_iapp_var_refs(source)
        apl_names = {r.apl_name for r in refs}
        assert "basic.addr" in apl_names
        assert "basic.port" in apl_names

    def test_extract_braced_var(self):
        source = "puts ${::basic__addr}\n"
        refs = extract_iapp_var_refs(source)
        assert len(refs) == 1
        assert refs[0].apl_name == "basic.addr"

    def test_no_match_for_non_iapp_vars(self):
        source = "set x $::env(HOME)\nset y $::simple_var\n"
        refs = extract_iapp_var_refs(source)
        # env(HOME) doesn't have __, simple_var doesn't have __
        assert len(refs) == 0

    def test_position_tracking(self):
        source = "set addr $::basic__addr\n"
        refs = extract_iapp_var_refs(source)
        assert len(refs) == 1
        assert refs[0].range.start.line == 0
        assert refs[0].range.start.character == 9  # position of $


class TestIappCrossFileDiagnostics:
    """Tests for cross-file presentation ↔ implementation diagnostics."""

    def setup_method(self) -> None:
        """iApp diagnostics gate on dialect — set ``f5-iapps`` per test.

        The production dispatcher in ``lsp/diagnostics_pipeline.py``
        only invokes these validators when ``_is_apl_source(uri)`` is
        true, which is correlated with the ``f5-iapps`` dialect.  These
        unit tests exercise the validators directly so we configure the
        dialect explicitly.  ``configure_signatures`` updates a
        ``ContextVar``; the per-method hook ensures every test starts in
        the right context regardless of pytest's collection order.
        """
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-iapps")

    def teardown_method(self) -> None:
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")

    def test_validators_silent_outside_iapp_dialects(self):
        """IAPP7001/7002/7003 must not fire in plain Tcl / iRules dialects."""
        from core.commands.registry.runtime import configure_signatures

        apl_source = "section basic {\n    string addr\n}\n"
        impl_source = "set port $::basic__port\n"  # undefined
        model = parse_apl(apl_source)
        refs = extract_iapp_var_refs(impl_source)
        try:
            for dialect in ("tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "f5-irules"):
                configure_signatures(dialect=dialect)
                impl_diags = validate_iapp_implementation(refs, model)
                pres_diags = validate_iapp_presentation(model, refs)
                iapp_codes = {d.code for d in impl_diags} | {d.code for d in pres_diags}
                assert not (iapp_codes & {"IAPP7001", "IAPP7002", "IAPP7003"}), (
                    f"IAPP* diagnostics leaked into {dialect}: {iapp_codes}"
                )
        finally:
            configure_signatures(dialect="f5-iapps")

    def test_undefined_variable_diagnostic(self):
        apl_source = "section basic {\n    string addr\n}\n"
        impl_source = (
            "set addr $::basic__addr\n"
            "set port $::basic__port\n"  # not defined in presentation
        )
        model = parse_apl(apl_source)
        refs = extract_iapp_var_refs(impl_source)
        # IAPP7001 is emitted by validate_iapp_implementation (positioned
        # in the implementation file), not validate_iapp_presentation.
        impl_diags = validate_iapp_implementation(refs, model)
        impl_codes = {d.code for d in impl_diags}
        assert "IAPP7001" in impl_codes
        # validate_iapp_presentation should not duplicate IAPP7001
        pres_diags = validate_iapp_presentation(model, refs)
        assert not any(d.code == "IAPP7001" for d in pres_diags)
        # IAPP7002 should not fire for basic.addr since it's referenced
        iapp7002_fields = [d for d in pres_diags if d.code == "IAPP7002"]
        assert not any("basic.addr" in d.message for d in iapp7002_fields)

    def test_unused_field_diagnostic(self):
        apl_source = (
            "section basic {\n    string addr\n    string port\n    string unused_field\n}\n"
        )
        impl_source = "set addr $::basic__addr\nset port $::basic__port\n"
        model = parse_apl(apl_source)
        refs = extract_iapp_var_refs(impl_source)
        diags = validate_iapp_presentation(model, refs)
        iapp7002 = [d for d in diags if d.code == "IAPP7002"]
        assert len(iapp7002) == 1
        assert "unused_field" in iapp7002[0].message

    def test_message_fields_not_flagged_as_unused(self):
        apl_source = 'section basic {\n    message "This is info"\n    string addr\n}\n'
        impl_source = "set addr $::basic__addr\n"
        model = parse_apl(apl_source)
        refs = extract_iapp_var_refs(impl_source)
        diags = validate_iapp_presentation(model, refs)
        # message fields should not trigger IAPP7002
        assert not any(d.code == "IAPP7002" and "message" in d.message.lower() for d in diags)

    def test_include_not_found_diagnostic(self):
        apl_source = '#include "nonexistent.apl"\nsection basic { string addr }\n'
        model = resolve_apl_includes(apl_source, "/tmp/nonexistent_dir_xyz")
        diags = validate_iapp_presentation(model)
        codes = {d.code for d in diags}
        assert "IAPP7003" in codes

    def test_no_diagnostics_without_impl(self):
        """Without implementation refs, only local checks are run."""
        apl_source = "section basic { string addr }\n"
        model = parse_apl(apl_source)
        diags = validate_iapp_presentation(model)
        # No IAPP7001 or IAPP7002 without impl refs
        assert not any(d.code in ("IAPP7001", "IAPP7002") for d in diags)

    def test_implementation_diagnostics(self):
        """validate_iapp_implementation produces diagnostics on impl source."""
        apl_source = "section basic { string addr }\n"
        impl_source = "set x $::basic__addr\nset y $::basic__missing\n"
        model = parse_apl(apl_source)
        refs = extract_iapp_var_refs(impl_source)
        diags = validate_iapp_implementation(refs, model)
        codes = {d.code for d in diags}
        assert "IAPP7001" in codes
        assert any("missing" in d.message for d in diags)
