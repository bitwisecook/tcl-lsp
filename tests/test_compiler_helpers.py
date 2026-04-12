"""Tests for shared compiler value/variable helper utilities."""

from __future__ import annotations

from core.compiler.value_shapes import is_pure_var_ref, parse_command_substitution
from core.compiler.var_refs import VarReferenceScanner, VarScanOptions


def test_is_pure_var_ref_shapes() -> None:
    assert is_pure_var_ref("$x")
    assert is_pure_var_ref("${x}")
    assert not is_pure_var_ref("$x y")
    assert not is_pure_var_ref("[set x 1]")


def test_parse_command_substitution_shapes() -> None:
    assert parse_command_substitution("[list a b]") == ("list", ("a", "b"))
    assert parse_command_substitution(" [clock seconds] ") == ("clock", ("seconds",))
    assert parse_command_substitution("[]") is None
    assert parse_command_substitution("clock seconds") is None


def test_var_reference_scanner_default_and_no_recurse_modes() -> None:
    source = "set out [list $x [set y $z]]"
    default_scanner = VarReferenceScanner()
    no_recurse_scanner = VarReferenceScanner(VarScanOptions(recurse_cmd_substitutions=False))

    assert default_scanner.scan_script(source) == {"x", "z"}
    assert no_recurse_scanner.scan_script(source) == set()


def test_var_reference_scanner_var_read_role_expansion() -> None:
    source = "parray arr\nvwait done\n"
    default_scanner = VarReferenceScanner()
    role_scanner = VarReferenceScanner(VarScanOptions(include_var_read_roles=True))

    assert default_scanner.scan_script(source) == set()
    assert role_scanner.scan_script(source) == {"arr", "done"}
