"""TP/TN regression family for VAR-as-command dispatch (post-stage2 §A).

Closes the prereq-A partials D3-P5 / D4-F6 / SF-1: a ``$cmd arg…`` dispatch
where ``$cmd`` provably holds a *known* command name (registered builtin or
user proc) must NOT fire W307, while a dispatch on a value SCCP proves is
*not* a command must still fire.

The inference is the existing SCCP CONST/CONSTSET lattice plus the D3-P2
call-site param-constant seeding (no separate ``known_command_names`` fact is
needed — a literal command name flows through the same constant lattice as any
other string).  The per-SSA-version refinement in
``analyser/_analyser/_diag_var_command.py`` reads the value at the dispatch's
exact use-version so a variable reassigned from a non-command to a command
before the dispatch is not conflated.

Ground truth: real C tclsh 9.0.3 (an ``invalid command name`` error is the TP
signal; a successful dispatch is the TN signal).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse


def _fires_w307(source: str) -> bool:
    return any(d.code == "W307" for d in analyse(source).diagnostics)


# --- Pair 1: local literal assignment of a user-proc / builtin name ---------


def test_var_as_cmd_literal_user_proc_silent():
    """TN: ``set cmd parse; $cmd x`` — ``parse`` is a user proc, so the
    dispatch is statically known.  No W307."""
    src = "proc parse {a} { return $a }\nproc d {} { set cmd parse; $cmd 5 }\n"
    assert not _fires_w307(src)


def test_var_as_cmd_literal_builtin_silent():
    """TN: ``set cmd incr; $cmd v`` — ``incr`` is a registered builtin."""
    src = "proc d {} { set v 0; set cmd incr; $cmd v }\n"
    assert not _fires_w307(src)


def test_var_as_cmd_literal_non_command_fires():
    """TP: ``set cmd notacommand; $cmd x`` — SCCP proves the value is the
    literal ``notacommand`` which is not a command.  tclsh errors
    ``invalid command name "notacommand"``."""
    src = "proc d {} { set cmd notacommand; $cmd 5 }\n"
    assert _fires_w307(src)


# --- Pair 2: interprocedural param flow (D3-P2 seeding) ---------------------


def test_var_as_cmd_param_flow_single_caller_silent():
    """TN: every caller passes the literal command name ``parse`` for the
    dispatched param, so the callee's ``$cmd`` is a known command."""
    src = "proc parse {a} { return $a }\nproc run {cmd} { $cmd 5 }\nproc d {} { run parse }\n"
    assert not _fires_w307(src)


def test_var_as_cmd_param_flow_non_command_fires():
    """TP: the only caller passes ``notacommand`` for the dispatched param."""
    src = "proc run {cmd} { $cmd 5 }\nproc d {} { run notacommand }\n"
    assert _fires_w307(src)


# --- Pair 3: phi join (branch-merged definitions) ---------------------------


def test_var_as_cmd_phi_two_commands_silent():
    """TN: both branches assign a known command, so the phi-joined value is
    a CONSTSET of all-known commands."""
    src = (
        "proc parse {a} { return $a }\n"
        "proc lint {a} { return $a }\n"
        "proc d {f} {\n"
        "    if {$f} { set c parse } else { set c lint }\n"
        "    $c 5\n"
        "}\n"
    )
    assert not _fires_w307(src)


def test_var_as_cmd_phi_command_and_non_command_fires():
    """TP: one branch assigns a non-command, so the phi-joined set contains
    a value that is not a command and the dispatch may be invalid."""
    src = (
        "proc parse {a} { return $a }\n"
        "proc d {f} {\n"
        "    if {$f} { set c parse } else { set c notacommand }\n"
        "    $c 5\n"
        "}\n"
    )
    assert _fires_w307(src)


# --- Pair 4: per-SSA-version precision (reassignment before dispatch) -------


def test_var_as_cmd_reassign_to_command_silent():
    """TN / per-SSA-version refinement: ``set c notacommand; set c parse;
    $c x`` — only the ``parse`` version reaches the dispatch, so the earlier
    ``notacommand`` write must not keep W307 alive.  Reading the use-version's
    SCCP value (not the merged per-function set) removes this false
    positive."""
    src = (
        "proc parse {a} { return $a }\n"
        "proc d {} {\n"
        "    set c notacommand\n"
        "    set c parse\n"
        "    $c 5\n"
        "}\n"
    )
    assert not _fires_w307(src)


def test_var_as_cmd_reassign_to_non_command_fires():
    """TP: the last write reaching the dispatch is a non-command."""
    src = (
        "proc parse {a} { return $a }\n"
        "proc d {} {\n"
        "    set c parse\n"
        "    set c notacommand\n"
        "    $c 5\n"
        "}\n"
    )
    assert _fires_w307(src)


# --- Conservatism: mixed callers stay silent (no FP cascade) ----------------


def test_var_as_cmd_mixed_callers_conservative_silent():
    """TN (intentionally conservative, per the §A acceptance criteria): when
    distinct callers pass different *known* command names, the CONSTSET is
    all-known and W307 stays silent — the dispatch is well-formed on every
    path."""
    src = (
        "proc parse {a} { return $a }\n"
        "proc lint {a} { return $a }\n"
        "proc run {c} { $c 5 }\n"
        "proc a {} { run parse }\n"
        "proc b {} { run lint }\n"
    )
    assert not _fires_w307(src)
