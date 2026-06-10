"""Tests for shared compiler value/variable helper utilities."""

from __future__ import annotations

from compiler.value_shapes import is_pure_var_ref, parse_command_substitution
from compiler.var_refs import (
    VarReferenceScanner,
    VarScanOptions,
    command_sub_write_names,
)


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


def test_command_sub_write_names() -> None:
    # catch result/options vars written inside a substitution.
    assert command_sub_write_names("[catch {error b} msg opts]") == {"msg", "opts"}
    assert command_sub_write_names("[catch {set x 1} m]") == {"m"}
    # regexp/scan literal match vars.
    assert command_sub_write_names("[regexp {(\\w+)} $s whole sub]") == {"whole", "sub"}
    assert command_sub_write_names("[scan $s {%d} a]") == {"a"}
    # Nested substitution.
    assert command_sub_write_names("[list [catch {x} e]]") == {"e"}
    # A catch buried in an expr body still writes its var (expr evaluates the
    # command substitution at run time).
    assert command_sub_write_names("[expr {[catch {eof $s} tmp] || $tmp}]") == {"tmp"}
    # A dynamically-named write target (`$dyn`) names a runtime variable, not a
    # literal write, so it is NOT collected (it would otherwise wrongly suppress
    # a genuine read of `dyn`).
    assert command_sub_write_names("[scan $s {%d} $dyn]") == set()
    # No command substitution → nothing.
    assert command_sub_write_names("catch {x} e") == set()
    assert command_sub_write_names("plain text") == set()
