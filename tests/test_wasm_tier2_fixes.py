"""Focused regression tests for the Tier-2 wasm-runtime / codegen fixes.

Each test pins one contract that previously truncated an entire
``tcl9-tcltest-wasm`` stem at the bundle's first trap.  See
``tests/baselines/tcl9-tcltest-wasm/README.md`` for the full
fix-and-ratchet workflow.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> tuple[str, str]:
    wasm = _compile_tcl(source)
    out = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
    stdout = out[1] if len(out) >= 2 else ""
    stderr = out[2] if len(out) >= 3 else ""
    return stdout, stderr


class TestSubstNonAsciiAfterDollar:
    """``$`` followed by a non-name byte must round-trip as a literal ``$``.

    parse-12.26 ``Tcl_ParseVarName [d2ffcca163] non-ascii``: with the bug,
    ``"$г"`` traps under ``can't read "": no such variable`` because
    the variable-name scanner consumed zero bytes after ``$`` and still
    fell through to ``var_resolve`` with an empty name TclObj.
    """

    def test_subst_dollar_non_ascii(self) -> None:
        # Route through ``subst`` so the runtime's ``subst_flagged_full``
        # path executes (the static codegen has its own ``$г`` lowering
        # that bypasses subst altogether).
        stdout, _ = _run('puts [subst "$\\u0433"]\n')
        assert stdout.strip() == "$г"

    def test_subst_dollar_dot_dollar(self) -> None:
        # ``$.`` and ``$$`` mirror parse-12.18: the dollar is literal
        # whenever the next byte does not start an identifier.
        stdout, _ = _run("puts [subst {$.$$}]\n")
        assert stdout.strip() == "$.$$"


class TestStringSubcommandUnderArity:
    """Static-codegen ``string <sub>`` calls must surface the runtime
    wrong-args error when the call is short of the registry-declared
    minimum, instead of zero-padding the missing slots and silently
    returning the empty string (error-1.3 / 1.6 / 1.7 / cmdAH-1.4 / 1.5).
    """

    def test_string_index_no_args(self) -> None:
        src = "catch {string index} b\nputs $b\nputs [info exists ::errorInfo]\n"
        stdout, _ = _run(src)
        lines = stdout.splitlines()
        assert lines[0] == 'wrong # args: should be "string index string charIndex"'
        # ``::errorInfo`` must be stamped by ``tcl_cmd_error`` even when
        # the failure originates from the static codegen path.
        assert lines[1] == "1"

    def test_string_index_stamps_errorinfo(self) -> None:
        # Static codegen path: ``string index`` with no operands must
        # raise the wrong-args error AND stamp ``::errorInfo``.  Before
        # the fix, the codegen called ``tcl_string_index(0, 0)``, which
        # silently returned the empty string with no ``::errorInfo``
        # update — error-1.3 / 1.6 / 1.7 then could not match the
        # expected error trace.
        src = "catch {string index} b\nputs $b\nputs [info exists ::errorInfo]\nputs $::errorInfo\n"
        stdout, _ = _run(src)
        lines = stdout.splitlines()
        assert lines[0] == 'wrong # args: should be "string index string charIndex"'
        assert lines[1] == "1"
        assert lines[2].startswith('wrong # args: should be "string index')


class TestArrayElementInEvalFallback:
    """Bare ``$arr(idx)`` round-tripped through the eval-fallback must
    keep its array-element semantics — ``word_piece``'s previous span-vs-
    text check classified bare ``$arr($cmd)`` as braced and re-emitted it
    as ``${arr($cmd)}``, collapsing the recursive ``$cmd`` substitution
    into a literal scalar lookup of the source spelling
    (cmdAH-1.4 / 1.5 ``$numargErrors($cmd)``).
    """

    def test_global_array_substituted_key(self) -> None:
        src = (
            'set ::numargErrors(KEY) "RESULT"\n'
            "proc x {cmd} {\n"
            "    variable numargErrors\n"
            "    catch {undef-cmd $numargErrors($cmd)} r\n"
            "    puts $r\n"
            "}\n"
            "x KEY\n"
        )
        stdout, _ = _run(src)
        # Without the fix: ``can't read "numargErrors($cmd)": no such variable``.
        assert stdout.strip() == 'invalid command name "undef-cmd"'


class TestTcltestInternalsBootstrap:
    """The ``[namespace which -command …] eq ""`` guard around
    ``namespace eval ::tcltest::internals { … }`` (cmdIL bootstrap)
    must evaluate the empty-string compare correctly when the LHS is
    a ``[cmd]`` substitution.  ``eval_string_expr`` previously only
    stripped ``{…}`` quoting from the operand spans, so ``""`` survived
    as the two-byte string ``""`` and never compared equal to the empty
    LHS.
    """

    def test_double_quoted_empty_eq(self) -> None:
        stdout, _ = _run('puts [expr {[namespace which -command ::nope] eq ""}]\n')
        assert stdout.strip() == "1"

    def test_double_quoted_literal_eq(self) -> None:
        stdout, _ = _run('set x abc\nputs [expr {$x eq "abc"}]\n')
        assert stdout.strip() == "1"


class TestErrorRecursionLimit:
    """``eval_proc_call_bucket`` must raise reference Tcl's
    ``too many nested evaluations`` once nested eval depth crosses
    the configured ceiling, instead of riding wasmtime's call stack
    until ``call stack exhausted`` aborts the bundle (error-1.8 was
    the canonical reproducer in the wider tcltest slice).
    """

    def test_recursion_limit_bounded_iteration(self) -> None:
        # Drive the eval-fallback path enough times to push past the
        # ceiling.  Each iteration parks one frame, so ``parked_top``
        # is the actual measure of nesting depth here.
        src = (
            "proc up {n} {\n"
            "    if {$n <= 0} { return ok }\n"
            "    uplevel 1 [list up [expr {$n - 1}]]\n"
            "}\n"
            "set rc [catch {up 200} msg]\n"
            "puts $rc\n"
            "puts $msg\n"
        )
        stdout, _ = _run(src)
        lines = stdout.splitlines()
        # rc=1 (caught error); the message must be the recursion-limit
        # diagnostic, not whatever leaked through from the wasm trap.
        assert lines[0] == "1"
        assert "too many nested evaluations" in lines[1]


class TestCatchBracedArgToProc:
    """A braced argument to a user proc inside a ``catch`` (or any
    implicit-return / tail position) must keep its literal bytes — braces
    suppress all substitution.  Previously ``_emit_call_stmt_tail`` pushed
    the arg through ``_emit_value`` without the ``was_braced`` flag the
    statement path uses, so ``catch {q {puts "$x $y"}} m`` re-substituted
    the brace-protected ``$x`` / ``$y`` and trapped under
    ``can't read "y": no such variable``.  This is the contract opt's
    ``::tcl::OptProc`` (``uplevel 1 [list ::proc ... "...$body"]`` wrapped
    in the caller's catch) depends on.
    """

    def test_catch_braced_dollar_arg_not_substituted(self) -> None:
        src = (
            "proc q {a} { return $a }\n"
            'catch {q {puts "$x $y"}} m\n'
            'puts "m=$m"\n'
        )
        stdout, _ = _run(src)
        assert stdout.strip() == 'm=puts "$x $y"'

    def test_catch_braced_arg_after_leading_stmt(self) -> None:
        # The proc call is the *last* (captured) statement of a
        # multi-statement catch body — the keep-result tail path.
        src = (
            "proc q {a} { return $a }\n"
            'catch {set zz 1\nq {puts "$x $y"}} m\n'
            'puts "m=$m"\n'
        )
        stdout, _ = _run(src)
        assert stdout.strip() == 'm=puts "$x $y"'


class TestUnsetRemovesVariableTrace:
    """Tcl drops a variable's traces when it is ``unset`` (after the
    unset callbacks fire).  The WASM runtime previously kept the trace
    registered, so a later variable that reused the name re-fired the
    stale callback — the trace-2.x cascade that eventually trapped via
    re-entrant xlinks lookup.  ``unset`` now removes scalar, array-name
    and array-element traces.
    """

    def test_scalar_trace_gone_after_unset(self) -> None:
        src = (
            "proc cb {args} { puts FIRED }\n"
            "trace add variable x write cb\n"
            "set x 1\n"
            "unset x\n"
            'puts "after=[trace info variable x]"\n'
            "set x 2\n"
            'puts "x=$x"\n'
        )
        stdout, _ = _run(src)
        # cb fires once (on ``set x 1``); after unset the trace is gone,
        # so ``set x 2`` is silent.
        assert stdout.splitlines() == ["FIRED", "after=", "x=2"]

    def test_local_trace_gone_after_unset(self) -> None:
        src = (
            "proc p {} {\n"
            "    proc cb {args} { puts FIRED }\n"
            "    trace add variable y write cb\n"
            "    set y 1\n"
            "    unset y\n"
            "    set y 2\n"
            '    puts "y=$y"\n'
            "}\n"
            "p\n"
        )
        stdout, _ = _run(src)
        assert stdout.splitlines() == ["FIRED", "y=2"]

    def test_array_element_trace_gone_after_whole_array_unset(self) -> None:
        # A stale element trace must not survive ``unset`` of the whole
        # array and re-fire on a re-created array (trace-1.6 leakage).
        src = (
            "proc cb {n1 n2 op} { puts STALE }\n"
            "set a(2) zzz\n"
            "trace add variable a(2) read cb\n"
            "set v $a(2)\n"
            "unset a\n"
            "set a(2) again\n"
            'puts "v2=$a(2)"\n'
        )
        stdout, _ = _run(src)
        # cb fires once on the first read; after ``unset a`` it's gone.
        assert stdout.splitlines() == ["STALE", "v2=again"]


class TestArrayDefault:
    """``array default set|get|exists|unset`` (Tcl 8.7/9, var-24.x): a
    per-array fallback returned when a missing element is read.  The
    default does not create elements — ``info exists`` / ``array size``
    ignore it — but element reads and read-modify-write ``incr`` observe
    it.
    """

    def test_default_get_and_read(self) -> None:
        src = (
            "array set ary {a 3}\n"
            "array default set ary 7\n"
            "puts [list $ary(a) $ary(b) [info exist ary(a)]"
            " [info exist ary(b)] [array default get ary]]\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "3 7 1 0 7"

    def test_default_exists_and_unset(self) -> None:
        src = (
            "array set ary {a 3}\n"
            "puts [array default exists ary]\n"
            "array default set ary 7\n"
            "puts [array default exists ary]\n"
            "array default unset ary\n"
            "puts [array default exists ary]\n"
            'set rc [catch {array default get ary} m]\n'
            'puts "$rc $m"\n'
        )
        stdout, _ = _run(src)
        assert stdout.splitlines() == ["0", "1", "0", "1 array has no default value"]

    def test_default_set_creates_empty_array(self) -> None:
        # ``array default set`` on a non-existent variable makes it an
        # empty array; the default doesn't add elements.
        src = (
            "array default set ary grill\n"
            'puts "[array size ary] [info exist ary(x)] [array exists ary]"\n'
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "0 0 1"

    def test_default_observed_by_incr(self) -> None:
        src = "array default set a 7\nincr a(x)\nputs $a(x)\n"
        stdout, _ = _run(src)
        assert stdout.strip() == "8"

    def test_default_dropped_on_unset(self) -> None:
        src = (
            "array default set a 7\n"
            "unset a\n"
            "puts [array default exists a]\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "0"


class TestAppendCreatesMissingVariable:
    """``append`` on an unset variable treats it as empty and creates it
    (Tcl semantics) rather than raising ``can't read "<var>": no such
    variable``.  The compiled ``append`` emitter read the variable
    strictly; it now uses the lenient read like ``lappend`` / ``incr``.
    """

    def test_append_missing_scalar(self) -> None:
        stdout, _ = _run("append s abc\nputs $s\n")
        assert stdout.strip() == "abc"

    def test_append_missing_scalar_no_error(self) -> None:
        stdout, _ = _run('set rc [catch {append s abc} m]\nputs "$rc $s"\n')
        assert stdout.strip() == "0 abc"

    def test_append_missing_array_element(self) -> None:
        stdout, _ = _run("set a(y) 1\nappend a(x) bar\nputs $a(x)\n")
        assert stdout.strip() == "bar"


class TestEnsembleUnknownRetry:
    """``namespace ensemble create -unknown HANDLER`` retry protocol: on
    an unknown subcommand the handler runs, then the ensemble re-resolves
    (the handler may have created the subcommand) and dispatches it.  A
    non-empty handler result is a rewritten command prefix invoked in
    place of the ensemble call.  (namespace-47.1/47.3.)
    """

    def test_handler_creates_then_retry(self) -> None:
        src = (
            "namespace eval ns {\n"
            "  namespace export *\n"
            "  proc mk {ens sub args} { proc $sub args { return done } }\n"
            "  namespace ensemble create -unknown ::ns::mk\n"
            "}\n"
            "puts [ns hello a b]\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "done"

    def test_handler_nonempty_rewrite(self) -> None:
        src = (
            "proc real {a b} { return \"real:$a:$b\" }\n"
            "namespace eval ns {\n"
            "  proc mk {ens sub args} { return [list ::real X] }\n"
            "  namespace ensemble create -unknown ::ns::mk\n"
            "}\n"
            "puts [ns foo Y]\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "real:X:Y"

    def test_handler_error_propagates(self) -> None:
        src = (
            "namespace eval ns {\n"
            "  proc mk {ens sub args} { return -code error \"no $sub\" }\n"
            "  namespace ensemble create -unknown ::ns::mk\n"
            "}\n"
            "puts [catch {ns zzz} m]\nputs $m\n"
        )
        stdout, _ = _run(src)
        assert stdout.splitlines() == ["1", "no zzz"]


class TestNamespaceEvalErrorInfo:
    """An error propagating out of ``namespace eval`` gets the
    ``(in namespace eval "::ns" script line N)`` errorInfo frame (the
    namespace-eval analogue of ``(procedure "X" line N)``), with the
    surrounding ``invoked from within`` callsite frame added by the
    interpreter.  (namespace-25.6/25.7.)  Driven through a dynamic
    ``eval`` so the body is interpreted (matching how tcltest runs test
    bodies) rather than compile-time inlined.
    """

    def test_namespace_eval_errorinfo_frame(self) -> None:
        src = (
            "namespace eval test_ns_1 {}\n"
            "set s {catch {namespace eval test_ns_1 {xxxx}} msg ; set ::errorInfo}\n"
            "puts [eval $s]\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == (
            'invalid command name "xxxx"\n'
            "    while executing\n"
            '"xxxx"\n'
            '    (in namespace eval "::test_ns_1" script line 1)\n'
            "    invoked from within\n"
            '"namespace eval test_ns_1 {xxxx}"'
        )


class TestEnsembleUnknownBadCode:
    """An ensemble ``-unknown`` handler that returns a non-ok / non-error
    code (break / continue / return) must raise ``unknown subcommand
    handler returned bad code: <name>`` rather than letting the control-
    flow signal leak out of the ensemble dispatch (namespace-47.4).
    """

    def test_break_from_unknown_handler(self) -> None:
        src = (
            "namespace eval ns {\n"
            "  proc Magic {e s args} { return -code break }\n"
            "  namespace ensemble create -unknown ::ns::Magic\n"
            "}\n"
            "puts [catch {ns spong} msg]\nputs $msg\n"
        )
        stdout, _ = _run(src)
        assert stdout.splitlines() == [
            "1",
            "unknown subcommand handler returned bad code: break",
        ]


class TestTraceCommandArity:
    """``trace`` argument/option validation (trace-14.0.x / 14.1-14.5):
    wrong argument counts and unknown options must raise the canonical
    Tcl diagnostics instead of silently succeeding."""

    def test_add_variable_wrong_args(self) -> None:
        stdout, _ = _run('puts [catch {trace add variable foo bar} m]\nputs $m\n')
        assert stdout.splitlines() == [
            "1",
            'wrong # args: should be "trace add variable name opList command"',
        ]

    def test_remove_variable_too_many_args(self) -> None:
        stdout, _ = _run('puts [catch {trace remove variable foo bar baz boo} m]\nputs $m\n')
        assert stdout.splitlines() == [
            "1",
            'wrong # args: should be "trace remove variable name opList command"',
        ]

    def test_info_variable_wrong_args(self) -> None:
        stdout, _ = _run('puts [catch {trace info variable foo bar} m]\nputs $m\n')
        assert stdout.splitlines() == [
            "1",
            'wrong # args: should be "trace info variable name"',
        ]

    def test_bad_option(self) -> None:
        stdout, _ = _run('puts [catch {trace gorp} m]\nputs $m\n')
        assert stdout.splitlines() == [
            "1",
            'bad option "gorp": must be add, info, or remove',
        ]

    def test_no_args(self) -> None:
        stdout, _ = _run('puts [catch {trace} m]\nputs $m\n')
        assert stdout.splitlines() == [
            "1",
            'wrong # args: should be "trace option ?arg ...?"',
        ]


class TestEnsembleParameters:
    """``namespace ensemble create -parameters {p ...}`` consumes that
    many words before the subcommand and re-inserts them ahead of the
    resolved implementation's arguments (namespace-53.1)."""

    def test_single_parameter(self) -> None:
        src = (
            "namespace eval ns {\n"
            "  namespace export x\n"
            "  proc x {para} {list 1 $para}\n"
            "  namespace ensemble create -parameters {para1}\n"
            "}\n"
            "puts [ns bar x]\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "1 bar"


class TestNamespaceUnknownHandler:
    """Per-namespace ``namespace unknown`` handler (namespace-52.x): a
    namespace's handler (or the root's, when it has none) intercepts an
    unknown command invoked within that namespace.  Queries report the
    explicit handler, defaulting to ``::unknown`` for the root and ``{}``
    elsewhere."""

    def test_set_and_dispatch(self) -> None:
        src = (
            "namespace eval foo {\n"
            "  namespace unknown [list dispatch]\n"
            "  proc dispatch {args} { return $args }\n"
            "  proc test {} { UnknownCmd a b c }\n"
            "}\n"
            "puts [foo::test]\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "UnknownCmd a b c"

    def test_query_defaults(self) -> None:
        src = (
            'puts "<[namespace eval foobar { namespace unknown }]>"\n'
            'puts "<[namespace eval :: { namespace unknown }]>"\n'
        )
        stdout, _ = _run(src)
        assert stdout.splitlines() == ["<>", "<::unknown>"]

    def test_global_handler_inherited(self) -> None:
        src = (
            "proc ::myunknown {args} { return \"MYUNKNOWN: $args\" }\n"
            "namespace eval :: { namespace unknown ::myunknown }\n"
            "set result [namespace eval foo { dummy a b c }]\n"
            "namespace eval :: { namespace unknown {} }\n"
            "puts $result\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "MYUNKNOWN: dummy a b c"


class TestEnsembleUnknownFqnName:
    """The ensemble ``-unknown`` handler receives the fully-qualified
    ensemble command name, not the short invoked word (namespace-47.7)."""

    def test_handler_gets_fqn(self) -> None:
        src = (
            "namespace ensemble create -command foo -unknown bar\n"
            "proc bar {args} { list ::set ::x [join $args |] }\n"
            "puts [foo {one two three}]\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "::foo|one two three"


class TestEnsembleConfigQuoting:
    """``namespace ensemble config`` renders each option value as a Tcl
    list element — a simple word is unbraced (``-unknown bar`` not
    ``-unknown {bar}``).  (namespace-47.5.)"""

    def test_unknown_word_unbraced(self) -> None:
        src = (
            "namespace ensemble create -command foo -unknown bar\n"
            "puts [dict get [namespace ensemble config foo] -unknown]\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "bar"


class TestTraceCommandTargetExists:
    """``trace add|remove|info command|execution`` requires the target
    command to exist, else ``unknown command "X"`` (trace-27.x/28.8/28.9)."""

    def test_info_command_missing(self) -> None:
        stdout, _ = _run('set rc [catch {trace info command thisdoesntexist} r]\nputs "$rc|$r"\n')
        assert stdout.strip() == '1|unknown command "thisdoesntexist"'

    def test_remove_execution_missing(self) -> None:
        stdout, _ = _run('set rc [catch {trace remove execution nope {enter} bar} r]\nputs "$rc|$r"\n')
        assert stdout.strip() == '1|unknown command "nope"'

    def test_existing_command_ok(self) -> None:
        stdout, _ = _run('proc foo {} {}\nset rc [catch {trace info command foo} r]\nputs "$rc|$r"\n')
        assert stdout.strip() == "0|"


class TestExecutionTraces:
    """``trace add execution`` enter/leave/enterstep/leavestep callbacks
    (trace-21.x and the broader execution-trace suite).  Driven through a
    dynamic ``eval`` so the traced proc's body is interpreted (step
    traces fire on each body command), matching how tcltest runs."""

    def test_enter_leave_enterstep_leavestep(self) -> None:
        body = (
            "proc traceExecute {args} { global info; lappend info $args }\n"
            "proc foo {x} {set b $x}\n"
            "set info {}\n"
            "trace add execution foo {enter leave enterstep leavestep} [list traceExecute foo]\n"
            "foo 3\n"
            "trace remove execution foo {enter leave enterstep leavestep} [list traceExecute foo]\n"
            "puts $info"
        )
        stdout, _ = _run("eval {" + body + "}\n")
        assert stdout.strip() == (
            "{foo {foo 3} enter} {foo {set b 3} enterstep} "
            "{foo {set b 3} 0 3 leavestep} {foo {foo 3} 0 3 leave}"
        )

    def test_trace_info_execution(self) -> None:
        src = (
            "eval {proc foo {} {}\n"
            "trace add execution foo {enter leave} bar\n"
            "puts [trace info execution foo]}\n"
        )
        stdout, _ = _run(src)
        assert stdout.strip() == "{{enter leave} bar}"


class TestTraceOpListValidation:
    """Trace op-lists are validated against the type's allowed ops; an
    invalid op, an abbreviation, or an empty list raises the canonical
    diagnostic (trace-14.6.x)."""

    def test_bad_op_variable(self) -> None:
        stdout, _ = _run('proc x {} {}\nset rc [catch {trace add variable x {y z w} a} m]\nputs "$rc|$m"\n')
        assert stdout.strip() == '1|bad operation "y": must be array, read, unset, or write'

    def test_null_op_list_command(self) -> None:
        stdout, _ = _run('proc x {} {}\nset rc [catch {trace add command x {} a} m]\nputs "$rc|$m"\n')
        assert stdout.strip() == '1|bad operation list "": must be one or more of delete or rename'

    def test_abbreviation_rejected(self) -> None:
        stdout, _ = _run('proc x {} {}\nset rc [catch {trace add variable x {r} a} m]\nputs "$rc|$m"\n')
        assert stdout.strip() == '1|bad operation "r": must be array, read, unset, or write'

    def test_execution_op_list(self) -> None:
        stdout, _ = _run('proc x {} {}\nset rc [catch {trace add execution x {bogus} a} m]\nputs "$rc|$m"\n')
        assert stdout.strip() == '1|bad operation "bogus": must be enter, leave, enterstep, or leavestep'
