"""Tests for the Option-shape factory specialisation pass (P8)."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.ir import IRBlock, IRCall, IRReturn, IRScript
from compiler.lowering import lower_to_ir
from compiler.passes.specialise_factories import (
    detect_factory_shape,
    specialise_factories,
)


class TestDetectFactoryShape:
    """Pattern detector (P8.1) — shapes that qualify and shapes that
    don't."""

    def test_minimal_factory_detected(self):
        source = textwrap.dedent("""\
            proc Configure {name default} {
                proc $name {{value {}}} [subst -nocommands {return $default}]
            }
        """)
        mod = lower_to_ir(source)
        shape = detect_factory_shape(mod.procedures["::Configure"])
        assert shape is not None
        assert shape.qualified_name == "::Configure"
        assert shape.params == ("name", "default")
        assert shape.name_param == "name"
        assert shape.child_params == "{value {}}"
        assert shape.child_body_template == "return $default"

    def test_multi_statement_body_rejected(self):
        # Prologue statements would be dropped by P8.2's no-op
        # replacement — the detector refuses to give the rewriter
        # anything to work with.
        source = textwrap.dedent("""\
            proc Configure {name default} {
                set helper {prologue}
                proc $name {{value {}}} [subst -nocommands {return $default}]
            }
        """)
        mod = lower_to_ir(source)
        assert detect_factory_shape(mod.procedures["::Configure"]) is None

    def test_name_must_be_a_param(self):
        # ``$other`` is a global/unknown var — doesn't pin to a
        # call-site arg, so the factory shape doesn't qualify.
        source = textwrap.dedent("""\
            proc Configure {name default} {
                proc $other {{value {}}} [subst -nocommands {return $default}]
            }
        """)
        mod = lower_to_ir(source)
        assert detect_factory_shape(mod.procedures["::Configure"]) is None

    def test_non_subst_body_rejected(self):
        # Body that isn't a ``[subst -nocommands …]`` command sub
        # shouldn't match the detector even if it's still a CMD
        # token.
        source = textwrap.dedent("""\
            proc Configure {name default} {
                proc $name {{value {}}} [format %s $default]
            }
        """)
        mod = lower_to_ir(source)
        assert detect_factory_shape(mod.procedures["::Configure"]) is None

    def test_nobackslashes_flag_refused(self):
        source = textwrap.dedent("""\
            proc Configure {name default} {
                proc $name {{value {}}} [subst -nobackslashes -nocommands {return $default}]
            }
        """)
        mod = lower_to_ir(source)
        assert detect_factory_shape(mod.procedures["::Configure"]) is None


class TestSpecialiseFactoriesCalls:
    """Call-site rewriter (P8.2) — end-to-end: factory + literal-args
    call sites produce specialised IRProcedures."""

    def test_literal_args_specialise_at_top_level(self):
        source = textwrap.dedent("""\
            proc Configure {name default} {
                proc $name {{value {}}} [subst -nocommands {return $default}]
            }
            Configure verbose 0
            Configure skip empty
        """)
        mod = lower_to_ir(source)
        specialise_factories(mod)
        assert "::verbose" in mod.procedures
        assert "::skip" in mod.procedures
        # Both child bodies contain the materialised ``return
        # <literal>``.
        verbose = mod.procedures["::verbose"]
        skip = mod.procedures["::skip"]
        assert verbose.body_source == "return 0"
        assert skip.body_source == "return empty"
        assert any(isinstance(s, IRReturn) for s in verbose.body.statements)
        assert any(isinstance(s, IRReturn) for s in skip.body.statements)
        # Call sites replaced by empty IRBlock no-ops.
        top = mod.top_level.statements
        assert isinstance(top[0], IRCall) and top[0].command == "proc"
        # The next two statements were ``Configure verbose 0`` /
        # ``Configure skip empty``; they should now be empty
        # IRBlocks.
        for i in (1, 2):
            block = top[i]
            assert isinstance(block, IRBlock)
            assert block.body == IRScript()

    def test_dynamic_arg_stays_on_runtime_path(self):
        # One literal + one ``$var`` at the call site means the
        # specialiser can't pin the full const-map; the call site
        # stays as an IRCall for runtime dispatch.
        source = textwrap.dedent("""\
            proc Configure {name default} {
                proc $name {{value {}}} [subst -nocommands {return $default}]
            }
            set dyn 0
            Configure verbose $dyn
        """)
        mod = lower_to_ir(source)
        specialise_factories(mod)
        assert "::verbose" not in mod.procedures
        # Top-level still has an IRCall to ``Configure``.
        assert any(
            isinstance(s, IRCall) and s.command == "Configure" for s in mod.top_level.statements
        )

    def test_no_factories_is_noop(self):
        source = textwrap.dedent("""\
            proc greet {x} {return "hi $x"}
            greet world
        """)
        mod = lower_to_ir(source)
        before = dict(mod.procedures)
        before_top = mod.top_level
        specialise_factories(mod)
        # Nothing added, nothing changed.
        assert dict(mod.procedures) == before
        assert mod.top_level is before_top

    def test_duplicate_name_last_wins(self):
        # ``Configure verbose 0; Configure verbose 1`` — both call
        # sites specialise to ``::verbose``; the second overwrites
        # the first.
        source = textwrap.dedent("""\
            proc Configure {name default} {
                proc $name {{value {}}} [subst -nocommands {return $default}]
            }
            Configure verbose 0
            Configure verbose 1
        """)
        mod = lower_to_ir(source)
        specialise_factories(mod)
        assert mod.procedures["::verbose"].body_source == "return 1"

    def test_child_proc_params_parsed(self):
        # Child proc params including a ``{name default}`` default-
        # value form.
        source = textwrap.dedent("""\
            proc Configure {name default} {
                proc $name {a {b 0} c} [subst -nocommands {return $default}]
            }
            Configure setter hello
        """)
        mod = lower_to_ir(source)
        specialise_factories(mod)
        assert mod.procedures["::setter"].params == ("a", "b", "c")

    def test_per_factory_cap_limits_specialisations(self):
        # P8.4: run with a small cap and verify the first N call
        # sites specialise but the (N+1)-th stays on the runtime
        # dispatch path.
        calls = "\n".join(f"Configure opt{i} val{i}" for i in range(5))
        source = (
            textwrap.dedent("""\
                proc Configure {name default} {
                    proc $name {{value {}}} [subst -nocommands {return $default}]
                }
            """)
            + calls
            + "\n"
        )
        mod = lower_to_ir(source)
        specialise_factories(mod, cap=3)
        # First three call sites specialised; last two still IRCall.
        synthesised = [f"::opt{i}" for i in range(5) if f"::opt{i}" in mod.procedures]
        assert len(synthesised) == 3
        remaining_calls = [
            s
            for s in mod.top_level.statements
            if isinstance(s, IRCall) and s.command == "Configure"
        ]
        assert len(remaining_calls) == 2

    def test_name_containing_colons_refused(self):
        # A literal name like ``my::verbose`` would need namespace-
        # scoped child registration; first wave refuses.
        source = textwrap.dedent("""\
            proc Configure {name default} {
                proc $name {{value {}}} [subst -nocommands {return $default}]
            }
            Configure my::verbose 0
        """)
        mod = lower_to_ir(source)
        specialise_factories(mod)
        assert "::my::verbose" not in mod.procedures
        # Call site remained an IRCall (not a no-op).
        assert any(
            isinstance(s, IRCall) and s.command == "Configure" for s in mod.top_level.statements
        )
