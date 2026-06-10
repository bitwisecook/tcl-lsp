"""Tests for the per-proc dependency fingerprint (Phase 6 foundation)."""

from __future__ import annotations

from compiler.lowering import lower_to_ir
from compiler.proc_fingerprint import dependency_fingerprint


def _fp(src: str, proc: str = "::f") -> set[str]:
    return set(dependency_fingerprint(lower_to_ir(src).procedures[proc]))


class TestDependencyFingerprint:
    def test_captures_called_commands_incl_substitutions(self):
        fp = _fp("proc f {x} { set y [helper $x]\n other $y\n return [compute $y] }")
        assert "cmd:helper" in fp
        assert "cmd:other" in fp
        assert "cmd:compute" in fp

    def test_captures_commands_in_nested_bodies(self):
        fp = _fp("proc f {x} { if {$x} { deep_cmd $x }\n while {$x} { loop_cmd } }")
        assert "cmd:deep_cmd" in fp
        assert "cmd:loop_cmd" in fp

    def test_captures_deps_in_control_flow_heads(self):
        # Conditions / foreach lists / switch subjects are external references
        # too; missing them under-approximates (the cache would go stale).
        assert "cmd:helper" in _fp("proc f {} { if {[helper]} { puts ok } }")
        assert "var:::flag" in _fp("proc f {} { if {$::flag} { puts ok } }")
        assert "cmd:more" in _fp("proc f {} { while {[more]} { step } }")
        assert "cmd:cond" in _fp("proc f {} { for {set i 0} {[cond]} {incr i} { x } }")
        assert "cmd:items" in _fp("proc f {} { foreach a [items] { puts $a } }")
        assert "cmd:pick" in _fp("proc f {} { switch [pick] { a {go} } }")

    def test_captures_global_refs(self):
        fp = _fp("proc f {} { return [expr {$::config + 1}] }")
        assert "var:::config" in fp

    def test_excludes_locals_and_params(self):
        fp = _fp("proc f {x} { set y 1\n return [expr {$x + $y}] }")
        # No local/param names leak in as dependencies.
        assert not any(d in ("var:x", "var:y") for d in fp)

    def test_stable_across_same_dependency_edits(self):
        # Body changes but the set of external symbols is identical -> same fp,
        # so per-proc diagnostic memoization would reuse the cached result.
        a = _fp("proc f {x} { return [helper $x] }")
        b = _fp("proc f {x} { set t [helper $x]\n return $t }")
        assert a == b

    def test_differs_when_dependency_changes(self):
        a = _fp("proc f {x} { return [helper $x] }")
        b = _fp("proc f {x} { return [DIFFERENT $x] }")
        assert a != b

    def test_order_independent(self):
        a = _fp("proc f {} { aa\n bb }")
        b = _fp("proc f {} { bb\n aa }")
        assert a == b
