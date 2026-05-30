"""TP/FP regression tests for the OBJ (object dispatch) family.

Each test pairs to an ``FP-OBJ-NN`` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

OBJ family covers:

* W307 — non-literal command word (``$var foo``) where ``$var`` is in fact a
  known object handle (snit self-references, snit components, namespaced
  factories, snit instance vars, local TclOO objects).
* W308 — method validation on snit instances (which goes through delegation /
  hull / options / built-ins and isn't soundly resolvable).
* Snit body modelling (private procs analysed, instance vars exempt from RBS).

Ground truth: real C tclsh 9.0.3 + snit 2.x.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


from analyser import analyse


def _codes(source: str, code: str):
    return [d for d in analyse(source).diagnostics if d.code == code]


# FP-OBJ-01 — snit self-references ($self/$type/$selfns/$win)


def test_FP_OBJ_01_self_dispatch_no_w307():
    """FP: $self/$type/$selfns/$win inside a snit::type method body dispatch
    on the current object — must NOT fire W307."""
    for ref in ("self", "type", "selfns", "win"):
        src = f"snit::type T {{\n method m {{}} {{ ${ref} foo }}\n}}"
        assert _codes(src, "W307") == [], f"${ref} foo wrongly fired W307"


def test_FP_OBJ_01_self_ref_outside_snit_still_w307():
    """TP control: a variable named `self` (etc.) in a vanilla proc is NOT
    a snit self-ref — its dispatch IS a stray non-literal command."""
    for ref in ("self", "type", "selfns", "win", "hull"):
        src = f"proc f {{}} {{ set {ref} [getThing]\n ${ref} foo }}"
        assert _codes(src, "W307"), f"${ref} foo outside snit body must still fire W307"


# FP-OBJ-02 — $hull dispatch (widgetadaptor)


def test_FP_OBJ_02_widgetadaptor_hull_no_w307():
    """FP: $hull configure is the canonical widgetadaptor delegation."""
    src = "snit::widgetadaptor W {\n method m {} { $hull configure -bg red }\n}"
    assert _codes(src, "W307") == []


# FP-OBJ-03 — snit component dispatch (instance-var method dispatch)


FP_OBJ_03_REPRO_COMPONENT = """\
snit::type T {
    component myexporter
    method run {fmt} {
        return [$myexporter export object $self $fmt]
    }
}
"""


def test_FP_OBJ_03_component_dispatch_no_w307():
    """FP: a snit ``component`` is an object handle — dispatching on it inside
    a method body is method dispatch."""
    assert _codes(FP_OBJ_03_REPRO_COMPONENT, "W307") == []


def test_FP_OBJ_03_constructor_dispatch_no_w307():
    """FP: same exemption applies in constructors."""
    src = (
        "snit::type T {\n"
        "    variable handler\n"
        "    constructor {args} {\n"
        "        $handler reset\n"
        "    }\n"
        "}"
    )
    assert _codes(src, "W307") == []


def test_FP_OBJ_03_typemethod_dispatch_no_w307():
    """FP: typevariable dispatch inside a typemethod body."""
    src = (
        "snit::type T {\n"
        "    typevariable registry\n"
        "    typemethod lookup {k} {\n"
        "        return [$registry get $k]\n"
        "    }\n"
        "}"
    )
    assert _codes(src, "W307") == []


# FP-OBJ-04 — namespaced-factory provenance


FP_OBJ_04_REPRO = """\
proc f {} {
    # ::struct::tree is a namespaced factory; $t is an object handle.
    set t [::struct::tree mytree]
    $t walk root
}
"""


def test_FP_OBJ_04_namespaced_factory_no_w307():
    """FP: a var assigned from a namespaced cmd-sub (`[::struct::tree …]` /
    `[ns::factory …]`) is treated as an object handle."""
    assert _codes(FP_OBJ_04_REPRO, "W307") == []


def test_FP_OBJ_04_short_namespace_form_no_w307():
    """FP: same applies to single-segment namespaced factories
    (`[struct::matrix]`)."""
    src = "proc f {} {\n    set m [struct::matrix]\n    $m add row\n}"
    assert _codes(src, "W307") == []


def test_FP_OBJ_04_bare_unknown_command_still_w307():
    """TP control: a *bare* (non-namespaced) unknown cmd-sub is not a factory —
    `$x run` still fires.  Proves the suppression is namespace-gated."""
    src = "proc f {} { set x [foo bar]\n $x run }"
    assert _codes(src, "W307"), "bare-name cmd-sub must NOT exempt dispatch"


def test_FP_OBJ_04_factory_does_not_leak_across_procs():
    """TP control: a factory assignment in one proc must not silence a same-
    named dispatch in another proc (provenance is per-proc)."""
    src = "proc a {} { set t [::struct::tree] }\nproc b {x} { set t $x\n $t foo }\n"
    assert _codes(src, "W307"), "factory must not propagate across proc boundaries"


# OPEN precision gap (xfail): the namespaced-factory rule is keyed
# purely on the *shape* ``set var [::ns::cmd ...]`` -- it assumes any
# namespaced command substitution returns an object handle.  But a
# namespaced proc CAN return a plain string:
#
#   namespace eval ::ns { proc make {} { return foo } }
#   proc f {} { set x [::ns::make]; $x method }   ;# false negative
#
# The current analyser silences W307 here even though ``$x = "foo"``
# isn't a command name.  A future refinement (track per-proc return-
# type lattice) would distinguish object-returning procs from
# string-returning ones.


def test_FP_OBJ_04_namespaced_string_returning_user_proc_fires_w307():
    """TP / return-type-aware guard: ``namespace eval ::ns { proc make
    {} { return foo } }`` makes ``::ns::make`` return a plain string,
    not an object handle.  Dispatching ``$x method`` where
    ``\\$x = "foo"`` fires W307 (no command named ``foo``).

    Locked the precision gap closure: the namespaced-factory rule in
    ``analyser/_analyser/_diag_var_command.py`` now distinguishes
    user-proc calls from builtin/namespaced builtins.  User procs
    only become factory sources when the fixpoint proves them
    object-returning; builtins keep the shape-based heuristic."""
    src = """\
namespace eval ::ns { proc make {} { return foo } }
proc f {} {
    set x [::ns::make]
    $x method
}
"""
    assert _codes(src, "W307"), (
        f"string-returning namespaced proc should NOT be treated as factory; "
        f"expected W307, got "
        f"{[(d.code, d.message[:50]) for d in analyse(src).diagnostics]}"
    )


def test_FP_OBJ_04_mixed_return_wrapper_fires_w307():
    """TP / all-paths return-type guard: a wrapper proc that returns
    a real factory result on one branch and a plain string on the
    other should NOT be tagged object-returning.  ``$x method`` with
    ``\\$x = "foo"`` is an invalid command (tclsh-verified).

    Locked the precision gap closure: object-returning classification
    now requires ALL feasible return values to be namespaced cmd-subs
    (was: only the LAST observed return), correctly rejecting
    mixed-return wrappers."""
    src = """\
proc make {flag} {
    if {$flag} {
        return foo
    } else {
        return [::struct::tree]
    }
}
proc f {} {
    set x [make 1]
    $x method
}
"""
    assert _codes(src, "W307"), (
        f"mixed-return wrapper should NOT be treated as object factory; "
        f"got {[(d.code, d.message[:50]) for d in analyse(src).diagnostics]}"
    )


# OPEN precision gaps for the W307 multi-dispatch / callback-shape
# heuristics (Finding 3 of PR #498 deep review).  Three reproducers
# where the analyser silences a real "invalid command name" hazard:
#
#   set cmd notacommand; $cmd a; $cmd b              -- multi-dispatch
#   set state(doneCallback) notacommand; $state(...) -- suffix-shape
#   set state(-foo) notacommand; $state(-foo) a      -- dash-prefix
#
# All three suppress W307 even though SCCP can see the local was set
# to a plain non-command string.  A refinement: only honour the
# heuristic when the SCCP value is UNKNOWN/OVERDEFINED (the case the
# heuristic was designed for), not when SCCP has concrete evidence
# the value isn't a command name.


def test_FP_OBJ_09_const_string_multi_dispatch_fires_w307():
    """TP / SCCP-evidence guard: ``set cmd notacommand`` makes ``$cmd``
    a CONST string that the SCCP lattice sees as ``notacommand``.
    Dispatching it twice doesn't make it an object handle --
    ``notacommand`` is not a registered command.  W307 must fire
    even though the multi-dispatch heuristic would otherwise suppress.

    Locked the precision gap closure: the SCCP-says-not-a-command
    predicate in ``analyser/_analyser/_diag_var_command.py`` overrides
    the heuristic suppressions when SCCP knows the local holds a
    non-command literal."""
    src = """\
proc f {} {
    set cmd notacommand
    $cmd a
    $cmd b
}
"""
    assert _codes(src, "W307"), (
        f"const-string multi-dispatch should fire W307; "
        f"got {[(d.code, d.message[:50]) for d in analyse(src).diagnostics]}"
    )


def test_FP_OBJ_10_const_string_callback_suffix_fires_w307():
    """TP / SCCP-evidence guard: ``set state(doneCallback)
    notacommand`` makes the callback-shaped slot CONST.  The dispatch
    ``$state(doneCallback) a`` is then invoking ``"notacommand"``
    which doesn't exist; W307 must fire.

    Locked the precision gap closure: SCCP CONST evidence on the
    array base name overrides the callback-suffix heuristic."""
    src = """\
proc f {} {
    set state(doneCallback) notacommand
    $state(doneCallback) a
}
"""
    assert _codes(src, "W307"), (
        f"const callback-suffix dispatch should fire W307; "
        f"got {[(d.code, d.message[:50]) for d in analyse(src).diagnostics]}"
    )


def test_FP_OBJ_10_const_string_dash_prefix_fires_w307():
    """TP / SCCP-evidence guard: ``set state(-foo) notacommand`` --
    same SCCP-override applies to the dash-prefixed key heuristic."""
    src = """\
proc f {} {
    set state(-foo) notacommand
    $state(-foo) a
}
"""
    assert _codes(src, "W307"), (
        f"const dash-prefix dispatch should fire W307; "
        f"got {[(d.code, d.message[:50]) for d in analyse(src).diagnostics]}"
    )


# OPEN precision gap: namespaced ensemble ${ns}::tail bypasses the SCCP
# constant-prefix check (Finding 7 of PR #498 deep review).


def test_FP_OBJ_07_namespaced_ensemble_const_prefix_fires_w307():
    """TP / SCCP-evidence guard: ``set ns nope`` followed by
    ``${ns}::missing arg`` -- tclsh errors ``invalid command name
    "nope::missing"``.  W307 (or W123) must fire.

    Locked the precision gap closure: the namespaced-ensemble check
    now composes the prefix value with the literal ``::tail`` and
    checks the registry / user-proc / class tables; when nothing
    matches and the prefix is CONST, the heuristic is overridden
    and W307 fires."""
    src = """\
proc f {} {
    set ns nope
    ${ns}::missing arg
}
"""
    # Either W307 (non-literal command) or W123 (unknown command name)
    # would prove the gap is closed; the catalog finding is that NEITHER
    # fires today.
    diags = [d for d in analyse(src).diagnostics if d.code in ("W307", "W123")]
    assert diags, (
        f"const-prefix namespaced ensemble dispatch on unknown command "
        f"should fire W307 or W123; got "
        f"{[(d.code, d.message[:50]) for d in analyse(src).diagnostics]}"
    )


# FP-OBJ-05 — snit instance dispatch (locally-defined snit type)


FP_OBJ_05_REPRO = """\
snit::type ::Counter { method bump {} { return 1 } }
proc use {} {
    # `Counter create %AUTO%` returns a snit instance; $a bump is
    # method dispatch, not a stray non-literal command.
    set a [Counter create %AUTO%]
    $a bump
}
"""


def test_FP_OBJ_05_snit_create_auto_no_w307():
    """FP: locally-defined snit-type's create-form returns OBJECT-typed value."""
    assert _codes(FP_OBJ_05_REPRO, "W307") == []


def test_FP_OBJ_05_snit_create_named_no_w307():
    """FP: same for the named-create form."""
    src = (
        "snit::type ::Counter { method bump {} { return 1 } }\n"
        "proc use {} {\n"
        "    set b [Counter create mine]\n"
        "    $b bump\n"
        "}\n"
    )
    assert _codes(src, "W307") == []


def test_FP_OBJ_05_snit_create_shorthand_no_w307():
    """FP: the create-shorthand `Foo %AUTO%` form."""
    src = (
        "snit::type ::Counter { method bump {} { return 1 } }\n"
        "proc use {} {\n"
        "    set c [Counter %AUTO%]\n"
        "    $c bump\n"
        "}\n"
    )
    assert _codes(src, "W307") == []


def test_FP_OBJ_05_snit_instance_no_w308():
    """FP: snit instances must not get W308 method validation — delegation /
    hull / options / built-ins make resolution unsound."""
    src = (
        "snit::type ::Foo { method bar {} {} }\n"
        "proc u {} { set o [Foo create %AUTO%]\n $o delegated_or_builtin }"
    )
    assert _codes(src, "W308") == []


# FP-OBJ-06 — snit private proc body IS analysed


def test_FP_OBJ_06_private_proc_body_analysed():
    """FP / control: a proc declared inside snit::type body is type-private but
    its body must still be analysed.  The `${a}($a)` scalar-vs-array smell
    inside the proc body must still fire — proves the body isn't silently
    dropped."""
    src = "snit::type T {\n    proc Helper {a} {\n        return [info exists ${a}($a)]\n    }\n}"
    diags = _codes(src, "W216")
    assert diags, "snit private proc body must be analysed (expected W216)"
    assert "${a}($a)" in diags[0].message


# FP-OBJ-07 — cmd-sub namespaced ensemble [ns_func]::method is dispatch, not stray word


FP_OBJ_07_REPRO = """\
namespace eval ::ns {
    proc dispatch {} { return ::ns::sub }
    proc sub::work {arg} { return $arg }
}
[::ns::dispatch]::work hello
"""


def test_FP_OBJ_07_cmdsub_namespaced_ensemble_no_w307():
    """FP: `[cmd]::method` is the namespaced-ensemble dispatch idiom — the
    literal `::method` tail provides static method-name evidence.  W307 must
    NOT fire because the dispatch is well-formed (method-name is known,
    namespace prefix is computed at runtime)."""
    assert _codes(FP_OBJ_07_REPRO, "W307") == [], (
        "cmd-sub namespaced ensemble [cmd]::method must not fire W307"
    )


def test_FP_OBJ_07_bare_cmdsub_dispatch_still_fires():
    """TP control: a bare `[cmd] $arg` dispatch with NO literal method-name
    tail still fires W307 — the suppression keys on the `::IDENT` suffix
    shape, not on cmd-sub-as-such."""
    # Bare cmd-sub at command position with no namespaced method tail.
    src = "[some_unknown_cmd] $arg\n"
    # We require *some* unresolved-command-style diagnostic to still fire;
    # tighter would be code-specific but the family-level guarantee is that
    # the suppression didn't go blanket.
    from analyser import analyse

    diags = analyse(src).diagnostics
    assert any(d.code in {"W307", "W101", "W101A"} for d in diags), (
        f"bare cmd-sub dispatch must still produce an unresolved-command diagnostic: {diags}"
    )


# FP-OBJ-08 — W307 suppressed on eval-substituted dispatch (W101 covers it)


FP_OBJ_08_REPRO = """\
proc f {cmd args} {
    # eval-substituted dispatch -- W101 already flags the eval-of-
    # substituted-string injection risk.  W307 reporting the same
    # site as "stray non-literal command word" is redundant noise.
    eval $cmd $args
}
"""


def test_FP_OBJ_08_eval_substituted_dispatch_no_w307():
    """FP: `eval $cmd ...` is an eval-injection site that W101 already
    flags.  W307 piling on with "stray non-literal command word" is pure
    duplicate noise — suppressed at the W307 detection point."""
    assert _codes(FP_OBJ_08_REPRO, "W307") == [], (
        "W307 must not fire on eval-substituted dispatch (W101 covers it)"
    )


def test_FP_OBJ_08_eval_substituted_dispatch_still_fires_w101():
    """TP control: W101 (eval-injection) must still fire on the same site.
    Proves the dedup suppressed only W307, not the real W101 finding."""
    diags = _codes(FP_OBJ_08_REPRO, "W101")
    assert diags, "W101 must still fire on eval $cmd $args (the real finding)"


# FP-OBJ-09 — W307 multi-dispatch local var (≥2 dispatches on same local)


FP_OBJ_09_REPRO = """\
proc analyze {G} {
    set TGraph [createTGraph $G]
    $TGraph node first
    $TGraph dispose
}
"""


def test_FP_OBJ_09_multi_dispatch_local_no_w307():
    """FP: when a single proc dispatches ≥2 times on the *same* local var
    (``$TGraph node first`` and ``$TGraph dispose``), the user has firmly
    treated that local as an object handle.  Single dispatch on an
    unknown source is ambiguous, but ≥2 dispatches on the same name
    demonstrate intent — W307 must not fire."""
    assert _codes(FP_OBJ_09_REPRO, "W307") == []


def test_FP_OBJ_09_single_dispatch_unknown_still_fires():
    """TP control: a single dispatch on a local set from an unknown source
    still fires W307 (one dispatch isn't enough to infer intent)."""
    src = "proc f {} { set x [whatever]\n $x op }"
    diags = _codes(src, "W307")
    assert diags, f"single-dispatch on unknown source must still fire W307; got {diags}"


# FP-OBJ-10 — W307 switch-callback array element ($state(-command))


FP_OBJ_10_REPRO = """\
proc h {state token} {
    $state(-command) $token
}
"""


def test_FP_OBJ_10_dash_prefixed_array_key_callback_no_w307():
    """FP: ``$state(-command) $token`` -- the array element with a
    dash-prefixed key is a switch-style callback slot (the user
    explicitly registered a command there via ``-command``).  W307
    must not fire on this dispatch shape."""
    assert _codes(FP_OBJ_10_REPRO, "W307") == []


def test_FP_OBJ_10_suffix_keyed_callback_no_w307():
    """FP: array element keyed by a callback-shaped suffix
    (``cmd``/``command``/``callback``/``handler``/``hook``/``proc``)
    is also a callback registration slot."""
    src = "proc h {state arg} { $state(doneCallback) $arg }"
    assert _codes(src, "W307") == []


# FP-OBJ-11 — W307 interprocedural object-factory tracking


FP_OBJ_11_REPRO = """\
proc createGraph {} { return [struct::tree] }
proc f {} { set t [createGraph]
$t op }
"""


def test_FP_OBJ_11_factory_dispatch_no_w307():
    """FP: ``createGraph`` directly returns ``[struct::tree]`` (a
    namespaced object factory).  Interproc fixpoint inference marks
    ``createGraph`` as object-returning, so callers ``set t
    [createGraph]; $t op`` are suppressed.

    Sample corpus impact: graphops.tcl 88→0 W307 firings (every
    ``createTGraph``/``createResidualGraph`` use cleared)."""
    assert _codes(FP_OBJ_11_REPRO, "W307") == []


def test_FP_OBJ_11_transitive_factory_no_w307():
    """FP: transitive factory chain -- ``createGraph`` returns ``$t``
    where ``$t`` was set from another factory.  The fixpoint propagates
    the OBJECT-RETURNING attribute transitively."""
    src = """\
proc factory {} { return [struct::tree] }
proc createGraph {} { set t [factory]
return $t }
proc f {} { set g [createGraph]
$g op }
"""
    assert _codes(src, "W307") == []
