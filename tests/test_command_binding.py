"""Tests for the flow-sensitive command-binding lattice.

The lattice answers "what does *this command name* resolve to at this program
point" so the optimiser can refuse to fold a builtin that was renamed/redefined.
These unit tests pin the lattice semantics directly (the optimiser-gating and
diagnostic layers are tested separately).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.command_binding import (
    BindingKind,
    analyse_command_binding,
    scan_command_mutations_in_bodies,
)
from compiler.compilation_unit import ensure_compilation_unit
from compiler.registry.runtime import configure_signatures


def _top(src: str):
    configure_signatures(dialect="tcl9.0")
    cu = ensure_compilation_unit(src, logger=None, context="t")
    assert cu is not None
    return cu, analyse_command_binding(cu.top_level.cfg)


def _end(cfg):
    """(block, idx) just past the last statement of the lexically-last block."""
    last = None
    for bname in cfg.reverse_postorder():
        stmts = cfg.blocks[bname].statements
        if stmts:
            last = (bname, len(stmts))
    assert last is not None
    return last


def _binding_at_command(res, cfg, canonical: str, query: str):
    """Binding of *query* at the first statement whose canonical cmd is *canonical*."""
    for bname in cfg.reverse_postorder():
        for idx, stmt in enumerate(cfg.blocks[bname].statements):
            if getattr(stmt, "canonical_command", None) == canonical:
                return res.binding_at(bname, idx, query)
    raise AssertionError(f"no statement with canonical_command {canonical!r}")


class TestBaseline:
    """With no rebinding, every name keeps its natural binding."""

    def test_builtins_are_builtin(self):
        cu, res = _top("set x 1\nputs [string length hi]\nincr x\n")
        b, i = _end(cu.top_level.cfg)
        for name in ("set", "string", "incr", "puts", "list", "lindex"):
            assert res.binding_at(b, i, name).kind is BindingKind.BUILTIN, name

    def test_user_proc_is_proc(self):
        cu, res = _top("proc sq {n} {return [expr {$n*$n}]}\nputs [sq 5]\n")
        b, i = _end(cu.top_level.cfg)
        binding = res.binding_at(b, i, "sq")
        assert binding.kind is BindingKind.PROC
        assert binding.target == "::sq"
        assert binding.is_foldable_proc

    def test_undefined_name_is_opaque_unknown(self):
        # A name no proc/builtin provides resolves via the ``unknown`` handler.
        cu, res = _top("nosuchcommand a b c\n")
        b, i = _end(cu.top_level.cfg)
        assert res.binding_at(b, i, "nosuchcommand").kind is BindingKind.OPAQUE


class TestFlowSensitiveRename:
    """The headline property: a rename only perturbs calls that follow it."""

    def test_proc_call_rename_call(self):
        # proc a ; call a ; rename a b ; call b
        src = "proc a {} {return 1}\na\nrename a b\nb\n"
        cu, res = _top(src)
        cfg = cu.top_level.cfg
        bname = next(iter(cfg.blocks))
        stmts = cfg.blocks[bname].statements
        # locate the two `a`-spelled / `b`-spelled calls and the rename
        call_a = next(i for i, s in enumerate(stmts) if s.command == "a")
        rename = next(i for i, s in enumerate(stmts) if s.command == "rename")
        call_b = next(i for i, s in enumerate(stmts) if s.command == "b")
        # before the rename, `a` is the proc, `b` is still unbound
        assert res.binding_at(bname, call_a, "a").kind is BindingKind.PROC
        assert res.binding_at(bname, call_a, "b").kind is BindingKind.OPAQUE
        # the rename event sees `a` still bound (state is *pre*-statement)
        assert res.binding_at(bname, rename, "a").kind is BindingKind.PROC
        # after the rename: `a` falls through to unknown, `b` is the old proc
        assert res.binding_at(bname, call_b, "a").kind is BindingKind.OPAQUE
        b_binding = res.binding_at(bname, call_b, "b")
        assert b_binding.kind is BindingKind.PROC and b_binding.target == "::a"

    def test_rename_deletion_is_opaque_not_error(self):
        # ``rename foo {}`` deletes foo; a later call hits ``unknown`` (opaque).
        cu, res = _top("proc foo {} {return 1}\nrename foo {}\nfoo\n")
        # the *last* `foo` (post-deletion) is opaque
        b, i = _end(cu.top_level.cfg)
        assert res.binding_at(b, i, "foo").kind is BindingKind.OPAQUE

    def test_builtin_rename_away_then_redefine_is_proc(self):
        # rename incr ::orig ; proc incr {…}: `incr` is now a *user proc*, so it
        # is no longer the foldable builtin, and ::orig holds the real builtin.
        src = "rename incr ::orig_incr\nproc incr {v} {upvar 1 $v x; ::orig_incr x 100}\nset c 0\n"
        cu, res = _top(src)
        b, i = _end(cu.top_level.cfg)
        assert res.binding_at(b, i, "incr").kind is BindingKind.PROC
        assert not res.binding_at(b, i, "incr").is_original_builtin
        assert res.binding_at(b, i, "::orig_incr").kind is BindingKind.BUILTIN


class TestRedefinition:
    def test_proc_redefined_stays_proc(self):
        # Two definitions of the same name: the name is still bound to *a* proc
        # (the later body wins).  The lattice records PROC; refusing to fold a
        # redefined proc is the separate ``redefined_procedures`` gate's job.
        cu, res = _top("proc f {} {return 1}\nproc f {} {return 2}\nf\n")
        b, i = _end(cu.top_level.cfg)
        binding = res.binding_at(b, i, "f")
        assert binding.kind is BindingKind.PROC
        assert binding.target == "::f"

    def test_proc_named_like_mathfunc_is_proc_not_builtin(self):
        # ``max`` is an expr math function but not a top-level command; defining
        # ``proc max`` binds a real, foldable proc (regression: it must not be
        # mistaken for redefining a builtin and collapsing to UNKNOWN).
        cu, res = _top("proc max {a b} {return $a}\nputs [max 1 2]\n")
        b, i = _end(cu.top_level.cfg)
        binding = res.binding_at(b, i, "max")
        assert binding.kind is BindingKind.PROC
        assert binding.target == "::max"


class TestInterpAlias:
    def test_alias_records_target(self):
        cu, res = _top("interp alias {} myset {} set\nmyset x 1\n")
        b, i = _end(cu.top_level.cfg)
        binding = res.binding_at(b, i, "myset")
        assert binding.kind is BindingKind.ALIAS
        assert binding.target == "::set"


class TestBranchJoin:
    def test_rename_in_one_arm_joins_to_unknown(self):
        # rename only on the true arm → after the merge the binding is ⊤.
        cu, res = _top("if {$x} {rename string ::s2}\nputs [string length hi]\n")
        binding = _binding_at_command(res, cu.top_level.cfg, "::puts", "string")
        assert binding.kind is BindingKind.UNKNOWN

    def test_rename_on_both_arms_agrees(self):
        # Same rename on both arms → the binding is deterministic (opaque) after.
        src = "if {$x} {rename string ::s2} else {rename string ::s2}\nputs [string length hi]\n"
        cu, res = _top(src)
        binding = _binding_at_command(res, cu.top_level.cfg, "::puts", "string")
        # string was renamed away on every path → opaque (auto-load), not ⊤
        assert binding.kind is BindingKind.OPAQUE


class TestDynamicMutation:
    def test_dynamic_rename_makes_everything_unknown(self):
        cu, res = _top("rename $cmd foo\nputs [string length hi]\n")
        binding = _binding_at_command(res, cu.top_level.cfg, "::puts", "string")
        assert binding.kind is BindingKind.UNKNOWN

    def test_dynamic_proc_name_makes_everything_unknown(self):
        cu, res = _top("proc $name {} {return 1}\nputs [string length hi]\n")
        binding = _binding_at_command(res, cu.top_level.cfg, "::puts", "string")
        assert binding.kind is BindingKind.UNKNOWN


class TestProcBodyMutationScan:
    """A rename buried in a proc body is summarised conservatively (module-wide)."""

    def test_nested_rename_in_proc_is_caught(self):
        cu, _ = _top("proc danger {} {if {$x} {rename string ::s2}}\nputs ok\n")
        cfgs = [fu.cfg for fu in cu.procedures.values()]
        mut = scan_command_mutations_in_bodies(cfgs)
        assert "::string" in mut.names
        assert not mut.dynamic
        assert not mut.trusts("string")
        assert mut.trusts("list")  # untouched names stay trusted

    def test_dynamic_rename_in_proc_distrusts_everything(self):
        cu, _ = _top("proc danger {} {rename $a $b}\nputs ok\n")
        cfgs = [fu.cfg for fu in cu.procedures.values()]
        mut = scan_command_mutations_in_bodies(cfgs)
        assert mut.dynamic
        assert not mut.trusts("string")
        assert not mut.trusts("anythingatall")

    def test_clean_module_trusts_all(self):
        cu, _ = _top("proc helper {x} {return [expr {$x+1}]}\nputs [helper 4]\n")
        cfgs = [fu.cfg for fu in cu.procedures.values()]
        mut = scan_command_mutations_in_bodies(cfgs)
        assert not mut.dynamic
        assert mut.names == frozenset()
        assert mut.trusts("string")
