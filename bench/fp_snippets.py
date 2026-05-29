"""Compiler-evidence generator for `docs/design/compiler/FP.md`.

Each FP-… entry in FP.md embeds a static text block showing the SSA defs/uses,
the SCCP `values` lattice, optional `types`, and the relevant analysis result
(``dead_stores`` / ``read_before_set`` / ``unused_variables`` / ``memory_ssa``)
for a tiny Tcl snippet — just enough to make the false-positive vs true-positive
verdict mechanically obvious.

The blocks are pasted statically into FP.md so reviewers don't have to run
anything; this script is how they get regenerated when the surrounding analysis
output evolves.  Each FP.md block names the regen command, so drift is visible.

The CLI registry lives at the bottom of this file; new determinations register
a callable returning a ``(label, source, render)`` triple.  Run:

  python -m bench.fp_snippets --id FP-RBS-03
  python -m bench.fp_snippets --list

The textual format is intentionally narrow (~80 cols) and free of timing /
non-deterministic state so the FP.md diff stays clean.
"""

from __future__ import annotations

import argparse
import sys
import textwrap
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.core_analyses import FunctionAnalysis  # noqa: E402
from compiler.ssa import SSAFunction  # noqa: E402
from tooling.explorer.pipeline import run_pipeline  # noqa: E402

# Reusable evidence-rendering primitives


def _stmt_op(stmt) -> str:
    """A short label for an SSA-wrapped IR statement (no positions, no repr)."""
    ir = getattr(stmt, "statement", stmt)
    cls = type(ir).__name__.removeprefix("IR")
    parts = [cls]
    name = getattr(ir, "name", None)
    if name:
        parts.append(repr(name))
    value = getattr(ir, "value", None)
    if value is not None and isinstance(value, str):
        v = value if len(value) <= 40 else value[:37] + "..."
        parts.append(f"value={v!r}")
    cmd = getattr(ir, "command", None) or getattr(ir, "command_name", None)
    if isinstance(cmd, str):
        parts.append(f"cmd={cmd!r}")
    return " ".join(parts)


def _term_op(term) -> str:
    if term is None:
        return "(none — fall-through exit)"
    cls = type(term).__name__.removeprefix("CFG")
    value = getattr(term, "value", None)
    cond = getattr(term, "condition", None) or getattr(term, "expr", None)
    if value is not None:
        return f"{cls} {value}"
    if cond is not None:
        return f"{cls} {cond}"
    return cls


def _format_defs(defs: dict[str, int]) -> str:
    if not defs:
        return "{}"
    return "{" + ", ".join(f"{k}#{v}" for k, v in defs.items()) + "}"


def _format_uses(uses) -> str:
    if not uses:
        return "{}"
    if isinstance(uses, dict):
        return "{" + ", ".join(f"{k}#{v}" for k, v in uses.items()) + "}"
    items = []
    for entry in uses:
        if isinstance(entry, tuple) and len(entry) == 2:
            items.append(f"{entry[0]}#{entry[1]}")
        else:
            items.append(str(entry))
    return "{" + ", ".join(items) + "}"


def _format_lattice(v) -> str:
    kind = getattr(v.kind, "name", str(v.kind))
    if v.value is not None:
        return f"{kind}({v.value!r})"
    if getattr(v, "values", None):
        sample = list(v.values)[:3]
        return f"{kind}({sample}{'…' if len(v.values) > 3 else ''})"
    return kind


def _format_type(t) -> str:
    """TypeLattice has a custom repr; trim the wrapper for readability."""
    s = repr(t).removeprefix("TypeLattice.of(").rstrip(")")
    return s.removeprefix("<TclType.").split(":", 1)[0]


@dataclass
class _Snapshot:
    name: str
    cfg: object
    ssa: SSAFunction
    analysis: FunctionAnalysis


def _pick(source: str, proc_qname: str, *, dialect: str = "tcl8.6") -> _Snapshot:
    """Return the snapshot for *proc_qname* (e.g. ``"::f"``) — analysed end-to-end."""
    result = run_pipeline(source, dialect=dialect)
    matches = [s for s in result.snapshots if s.name == proc_qname]
    if not matches:
        names = [s.name for s in result.snapshots]
        raise SystemExit(f"proc {proc_qname!r} not found in snapshots {names}")
    s = matches[0]
    return _Snapshot(name=s.name, cfg=s.cfg, ssa=s.ssa, analysis=s.analysis)


def render_evidence(
    snap: _Snapshot,
    *,
    vars_of_interest: Sequence[str] = (),
    show: Iterable[str] = ("ssa", "values", "rbs", "dead", "unused"),
) -> str:
    """Render the textual evidence block for a FP.md entry.

    *vars_of_interest* names the variables whose lattice/type entries should be
    pulled out — typically the variable the FP/TP determination hinges on.  The
    SSA listing always includes every defs/uses; lattice/type listings are
    filtered to *vars_of_interest* when set (else show all).
    """
    show = set(show)
    out: list[str] = []
    out.append(f"function {snap.name}")
    if "ssa" in show:
        for block in snap.cfg.blocks.values():
            ssa_block = snap.ssa.blocks.get(block.name)
            out.append(f"  block {block.name}")
            if ssa_block and ssa_block.phis:
                for phi in ssa_block.phis:
                    out.append(f"    phi  {phi}")
            stmts = ssa_block.statements if ssa_block else block.statements
            for i, stmt in enumerate(stmts):
                defs = _format_defs(getattr(stmt, "defs", {}) or {})
                uses = _format_uses(getattr(stmt, "uses", ()) or ())
                out.append(f"    [{i}] {_stmt_op(stmt)}  defs={defs}  uses={uses}")
            out.append(f"    term {_term_op(block.terminator)}")

    def _wanted(key: tuple) -> bool:
        return not vars_of_interest or key[0] in vars_of_interest

    if "values" in show:
        lat = snap.analysis.values
        entries = [(k, v) for k, v in sorted(lat.items()) if _wanted(k)]
        if entries:
            out.append("  values (SCCP lattice)")
            for (name, ver), v in entries:
                out.append(f"    {name}#{ver}: {_format_lattice(v)}")
    if "types" in show:
        types = getattr(snap.analysis, "types", None) or {}
        entries = [(k, t) for k, t in sorted(types.items()) if _wanted(k)]
        if entries:
            out.append("  types")
            for (name, ver), t in entries:
                out.append(f"    {name}#{ver}: {_format_type(t)}")
    if "rbs" in show:
        rbs = getattr(snap.analysis, "read_before_set", ()) or ()
        if rbs:
            out.append("  read_before_set")
            for r in rbs:
                out.append(f"    {r}")
        else:
            out.append("  read_before_set: (none)")
    if "dead" in show:
        ds = getattr(snap.analysis, "dead_stores", ()) or ()
        if ds:
            out.append("  dead_stores")
            for d in ds:
                out.append(f"    {d}")
        else:
            out.append("  dead_stores: (none)")
    if "unused" in show:
        u = getattr(snap.analysis, "unused_variables", ()) or ()
        if u:
            out.append("  unused_variables")
            for entry in u:
                out.append(f"    {entry}")
    return "\n".join(out)


# Snippet registry — each entry produces the exact evidence pasted into FP.md.


@dataclass
class _Entry:
    label: str
    source: str
    proc: str
    vars: tuple[str, ...] = ()
    show: tuple[str, ...] = ("ssa", "values", "rbs", "dead", "unused")
    dialect: str = "tcl8.6"
    notes: str = ""


def _dedent(s: str) -> str:
    return textwrap.dedent(s).strip("\n") + "\n"


ENTRIES: dict[str, _Entry] = {}


def register(fp_id: str, entry: _Entry) -> None:
    if fp_id in ENTRIES:
        raise SystemExit(f"duplicate FP-id {fp_id!r}")
    ENTRIES[fp_id] = entry


# PR 0 / NAB family — confirm-correct audits.  These reproduce the *audited*
# construct and prove the analyser handles it correctly (no FP, no missed real
# bug).  Each TP/FP test in tests/test_fp_nab.py uses the same source.

register(
    "FP-NAB-01",
    _Entry(
        label="lset append-slot (index == length) is legal, NOT W231",
        proc="::f",
        vars=("l",),
        show=("ssa", "values", "dead", "unused"),
        notes=(
            "tclsh-verified: `lset l 3 X` on a 3-element list APPENDS X (not an error).\n"
            "Analyser path: analyser/checks/_bounds.py:318 uses `resolved > list_len`,\n"
            "which correctly permits index==len.  The confirm-correct audit dates to the\n"
            "Phase-3 interval-bounds rewire.  Uses a parameter so the proc has no\n"
            "incoming SSA dead-store noise from a `set` immediately overwritten by lset."
        ),
        source=_dedent(
            """
            proc f {l} {
                # contract: caller passes a 3-element list, e.g. {a b c}
                lset l 3 X      ;# index==len -> APPEND, not out-of-range
                return $l
            }
            """
        ),
    ),
)

register(
    "FP-NAB-02",
    _Entry(
        label='lindex out-of-range returns "" — smell (W230), not an error',
        proc="::top",
        vars=(),
        show=("ssa", "values", "dead", "unused"),
        notes=(
            "tclsh-verified: `lindex {a b c} 9` returns the empty string (NO error).\n"
            "The W230 smell-only severity is the verdict; W231 (lset) is stronger because\n"
            "the same out-of-range index THERE is a real tclsh error.  W230's syntactic\n"
            "check fires only when both the list AND the index are literals on the same\n"
            "call (analyser/checks/_bounds.py:118-181); the dynamic interval path covers\n"
            "the variable-routed case once value-flow is tightened."
        ),
        source=_dedent(
            """
            # Top-level lindex with literal list + literal out-of-range index.
            # tclsh returns "" silently — likely-bug, not an error.
            set x [lindex {a b c} 9]
            return $x
            """
        ),
    ),
)

register(
    "FP-NAB-03",
    _Entry(
        label="Phase-4 interproc SCC NOT NEEDED — recursive procs are already detected pure",
        proc="::fact",
        vars=(),
        show=("ssa",),
        notes=(
            "Original concern: recursive procs conservatively marked impure.  Audit\n"
            "(`tmp/algo_experiments.py` EXP4) verified `analyse_interprocedural_ir` already\n"
            "uses an order-independent worklist fixpoint: purity is the greatest fixpoint\n"
            "(optimistic init + monotone-decreasing); effects the least fixpoint.  Mutually-\n"
            "recursive pure procs (`ev`/`od`) come out `pure=True` already.  An SCC\n"
            "condensation pass would yield zero precision gain on an 8-SCC corpus and only\n"
            "risk — confirmed-correct, no change."
        ),
        source=_dedent(
            """
            proc fact {n} {
                if {$n <= 1} { return 1 }
                return [expr {$n * [fact [expr {$n - 1}]]}]
            }
            """
        ),
    ),
)


# PR 1 / RBS family — read-before-set (W210/W213/W214) determinations.

register(
    "FP-RBS-01",
    _Entry(
        label="info exists / array exists is the test-before-use idiom (not W210)",
        proc="::maybe_get",
        vars=("v",),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "Tcl idiom: `if {[info exists v]} { … $v … }` legally reads $v only when\n"
            "the variable is set.  tclsh-verified: `info exists undef` returns 0 with\n"
            "no error, while a bare `$undef` raises 'no such variable'.  Pre-fix W210\n"
            "(commits 9b73053, c5a23d5) wrongly flagged the guarded read; the fix\n"
            "exempts names appearing inside `info exists` / `array exists` calls (incl.\n"
            "nested in EXPR/BODY scripts) — see `existence_test_names` in\n"
            "compiler/var_refs.py and the guard in _read_before_set."
        ),
        source=_dedent(
            """
            proc maybe_get {} {
                # v is never set in this proc — the info-exists guard is the entire
                # safety: a bare `$v` here would be a hard tclsh error.
                if {[info exists v]} { return $v }
                return {}
            }
            """
        ),
    ),
)


register(
    "FP-RBS-02",
    _Entry(
        label="catch/regexp/scan command-sub writes are not read-before-set",
        proc="::f",
        vars=("err",),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "Tcl semantics: `catch {…} msg ?opts?` writes msg/opts in the *caller*\n"
            "scope (tclsh-verified).  Same for `regexp -> …` match-vars and `scan` output\n"
            "vars.  But the whole `[catch {…} err]` is one opaque value word, so SSA\n"
            "never records `err` as a def → a later $err read showed up as a spurious\n"
            "W210 'read before set'.  Fix (commit 4e4316b): `command_sub_write_names` in\n"
            "compiler/var_refs.py recovers literal VAR_WRITE targets inside command subs\n"
            "(excluding dynamic `$name` targets — those name a runtime variable, a read of\n"
            "the name var, not a literal write); `_read_before_set` exempts those names."
        ),
        source=_dedent(
            """
            proc f {} {
                # [catch …] writes 'err' in this scope (tclsh-verified);
                # the read in the consequent must NOT be W210.
                if {[catch {operation} err]} { puts "failed: $err" }
            }
            """
        ),
    ),
)


register(
    "FP-RBS-03",
    _Entry(
        label="frozen-loop bodies (while/for with cmd-sub condition) — body writes recovered",
        proc="::f",
        vars=("line", "n"),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "A `while`/`for` whose condition is a command substitution\n"
            "(`while {[gets $fp line] >= 0}`, `while {[llength $l]}`) is kept as an\n"
            "opaque barrier so default-bytecode codegen stays tclsh-parity.  The pre-fix\n"
            "analyser recovered body *reads* but not body *writes* — so every body-local\n"
            "var (loop accumulators / temporaries) looked read-before-set.  Commit\n"
            "e319c3a added `body_write_names` to balance the recovery: VAR_WRITE targets +\n"
            "foreach/lmap loop vars + recursion into BODY-role scripts."
        ),
        source=_dedent(
            """
            proc f {fp} {
                # gets writes 'line' AND the body sets 'n' — both are body-local
                # but the frozen-loop body keeps them invisible to SSA defs.
                while {[gets $fp line] >= 0} {
                    set n [string length $line]
                    puts "$line ($n chars)"
                }
            }
            """
        ),
    ),
)


register(
    "FP-RBS-04",
    _Entry(
        label="qualified variable aliases (variable ${name}::tail) — local name is the tail",
        proc="::ns::get",
        vars=("graphAttr",),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "tclsh-verified: `variable ns::children` (or the dynamic-namespace form\n"
            "`variable ${name}::children`) declares a LOCAL alias whose name is the\n"
            "**static tail** (`children`).  Pre-fix the def was recorded under the\n"
            "qualified spelling and the `$`-prefixed dynamic form was filtered out\n"
            "of `variable_declaration_indices`, leaving the tail un-defed.  Result:\n"
            "every read of $children / $graphAttr(...) fired W210 (and W213 for the\n"
            "matching `unset`).  Fix (commit 6207fe0): `_read_before_set` exempts the\n"
            "static tail via `_qualified_variable_alias_tails`, plus broadens the\n"
            "namespace skip from `name.startswith('::')` to `'::' in name`."
        ),
        source=_dedent(
            """
            proc ::ns::get {name key} {
                # `variable ${name}::graphAttr` declares the local alias 'graphAttr';
                # the qualified form is just where the storage lives.
                variable ${name}::graphAttr
                if {![info exists graphAttr($key)]} { return "" }
                return $graphAttr($key)
            }
            """
        ),
    ),
)


register(
    "FP-RBS-05",
    _Entry(
        label="namespace upvar alias-not-a-def (OPEN; ~39 W210 still false-positive)",
        proc="::tester",
        vars=("alias",),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "OPEN finding (see docs/design/compiler/review-findings-deferred.md and the\n"
            "plan ledger).  `namespace upvar ns src alias` legally links `alias` to\n"
            "`ns::src` in the caller frame (tclsh-verified) — a true def of the local.\n"
            "But `lower_upvar` (compiler/lowering_hooks/_var.py) only registers `upvar`'s\n"
            "alias as an IRCall def; `namespace upvar` has no symmetric hook (its command\n"
            "is `namespace`, dispatched on subcommand later), so `alias` is left undefined\n"
            "→ false W210.\n\n"
            "A `lower_namespace_upvar` hook returning None for non-`upvar` subcommands\n"
            "(so `namespace eval` still falls through) cleanly fixes RBS but feeds the\n"
            "shimmer pass: shimmer defaults an unknown intrep to STRING and flags the\n"
            "first list/dict op on the alias, adding ~16 aycock FPs + unmasking ~241\n"
            "pre-existing upvar-alias shimmer (safe.tcl `state` dominates).  Suppressing\n"
            "shimmer on alias names is sound (alias intrep is externally determined —\n"
            "same principle as SCCP force-OVERDEFINED-for-escaping), but the trade-off\n"
            "is a policy call needing review.  Land together once decided."
        ),
        source=_dedent(
            """
            proc tester {} {
                # tclsh: 'alias' is now the caller-scope name for ::ns::state.
                namespace upvar ::ns state alias
                return $alias
            }
            """
        ),
    ),
)


register(
    "FP-RBS-06",
    _Entry(
        label="catch's output-var inside an expr body is written during expr eval",
        proc="::f",
        vars=("tmp", "eof"),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "tclsh-verified: `[expr {[catch {…} tmp] || $tmp}]` runs the cmd-sub during\n"
            "expr eval, so `tmp` is written before the `|| $tmp` subexpression reads it.\n"
            "FP-RBS-02 (commit 4e4316b) handled `[catch …]` at the *command-arg* level;\n"
            "this entry's extension (commit 6ae85f4) walks IRAssignExpr/IRExprEval/\n"
            "IRReturn expr ASTs collecting cmd-sub-write targets there too, so\n"
            "`statement_cmd_sub_write_names` covers EXPR-role args as well as plain\n"
            "command args."
        ),
        source=_dedent(
            """
            proc f {sock} {
                # http.tcl:4340 pattern: the [catch …] inside [expr {…}] writes
                # 'tmp' during expr eval; the `|| $tmp` read must not be W210.
                set eof [expr {[catch {eof $sock} tmp] || $tmp}]
                return $eof
            }
            """
        ),
    ),
)


register(
    "FP-RBS-07",
    _Entry(
        label="dynamically-named namespace eval bodies are still analysed (inner procs not opaque)",
        proc="::greet",
        vars=("who",),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "Pre-fix: a `namespace eval [expr-or-cmd-sub] { … }` whose name is computed\n"
            "was a fully opaque IRBlock barrier — inner `proc` bodies were never\n"
            "analysed, so their params leaked as W210.  Fix (commit cb14411): when the\n"
            "body is static (a literal braced script) the lowerer inline-compiles it\n"
            "(procs lifted to their own scope) using the enclosing namespace as a\n"
            "best-effort name, preserving the original IRCall for codegen so bytecode\n"
            "stays byte-identical."
        ),
        source=_dedent(
            """
            # logger.tcl:1007-1016 pattern: ${service} is the enclosing proc's
            # parameter; the dynamic namespace name doesn't stop the body's inner
            # `proc greet` from being analysed (post-fix).
            proc trace_on {service} {
                namespace eval ::logger::tree::${service} {
                    proc greet {who} { return "hello $who" }
                }
            }
            """
        ),
    ),
)


register(
    "FP-RBS-08",
    _Entry(
        label="upvar with a dynamic target (upvar 1 $name var) is a real alias-def",
        proc="::f",
        vars=("var",),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "tclsh-verified: `upvar 1 $name var` aliases the local `var` to a caller-\n"
            "scope variable named by the runtime value of `$name`.  Pre-fix the escaping-\n"
            "name collector skipped pairs whose *target* started with `$`, so writing the\n"
            "alias was wrongly W220/W211 and reading it was wrongly W210.  Fix (commit\n"
            "9f15e05): added `allow_dynamic_target` to `upvar_local_declaration_indices`\n"
            "so the escaping path opts in (memory-SSA / definition callers keep strict\n"
            "matching since they need the resolved target).  Companion fix: IRBlock\n"
            "use-extraction now reads `source_args[1]` so `namespace eval $ns {…}`'s\n"
            "`$ns` name is recovered as a read."
        ),
        source=_dedent(
            """
            proc f {name} {
                # picoirc.tcl:69 pattern: upvar 1 $context irc — aliases 'irc'
                # to whatever the caller named.  Writes + reads must be silent.
                upvar 1 $name var
                set var 99
                return $var
            }
            """
        ),
    ),
)


register(
    "FP-RBS-09",
    _Entry(
        label="for-init + regexp/cmd-sub captures inside un-lowered switch arms",
        proc="::f",
        vars=("j", "v"),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "Un-lowered switch arms (treated as opaque IRBlock for body-script analysis)\n"
            "surfaced `for {set j 0} …`'s init def and `if {[regexp … -> v]} …`'s capture\n"
            "as false read-before-set.  Fix (commit 9e379bd, local-only): completed the\n"
            "def set inside `_free_reads_in_ir_script` (`_collapsed_extra_defs`) — for-init\n"
            "and for-next + condition cmd-sub defs are now recovered name-level.  Shared\n"
            "`cfg._defs_from_ir_script` left untouched to avoid bigfloat2 S100 shift."
        ),
        source=_dedent(
            r"""
            proc f {n} {
                switch -- $n {
                    a {
                        for {set j 0} {$j < 3} {incr j} { puts $j }
                    }
                    b {
                        if {[regexp {(\w+)} "foo" -> v]} { puts $v }
                    }
                }
            }
            """
        ),
    ),
)


register(
    "FP-RBS-10",
    _Entry(
        label="eval / namespace eval literal-body reads are recovered",
        proc="::f",
        vars=("x",),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "tclsh-verified: `eval {puts $x}` evaluates the braced body in the *current*\n"
            "scope, so `$x` is a real read of the local.  Pre-fix the eval/namespace-eval\n"
            "body was an opaque IRBlock barrier — `x` looked unused (W211) and any\n"
            "body-only var looked read-before-set.  Fix (commit 6f69c86): suppress-only\n"
            "name-level recovery via `_extra_local_reads` + `_block_local_reads` (public\n"
            "`ssa.statement_read_names`).  Full flatten-into-CFG remains a future option."
        ),
        source=_dedent(
            """
            proc f {x} {
                # eval's braced body evaluates in *this* scope: $x is a real read of
                # the parameter, so 'x' must not be reported W211 ("unused").
                eval { puts $x }
            }
            """
        ),
    ),
)


register(
    "FP-RBS-11",
    _Entry(
        label="qualified-builtin loops (::foreach / ::lmap / ::for / ::while)",
        proc="::f",
        vars=("k", "v"),
        show=("ssa", "rbs", "dead", "unused"),
        notes=(
            "tclsh-verified: `::foreach` is fully equivalent to `foreach` (it's just the\n"
            "absolutely-qualified spelling).  Pre-fix the lowering dispatch keyed on\n"
            "bare names, so a qualified call (common in tcllib: html.tcl:153 uses\n"
            "`::foreach {vars vals} …`) stayed an opaque IRCall whose loop vars and body\n"
            "were invisible → ~80 W210 FPs.  The proper lowering fix surfaced collateral\n"
            "in other checks (dict-internals E002, html shimmer), so the analysis-only\n"
            "fix (commit 2f67c93) extended the un-lowered-loop recovery in\n"
            "`_read_before_set` to recover ::foreach / ::lmap loop vars + body writes\n"
            "name-level — same W210 win (−29 full corpus) without exposing the body."
        ),
        source=_dedent(
            """
            proc f {dict} {
                # html.tcl:153 pattern: ::foreach is just qualified foreach.
                # Loop vars k,v and body reads must all be silent.
                ::foreach {k v} $dict { puts "$k=$v" }
            }
            """
        ),
    ),
)


# PR 2 / DS family — dead-store / unused (W220/W211) determinations.


register(
    "FP-DS-01",
    _Entry(
        label="incr/append/lappend inside cmd-sub: read-modify-write keeps init live",
        proc="::f",
        vars=("i",),
        show=("ssa", "dead", "unused"),
        notes=(
            "Tcl semantics: `incr i $j` is a *read-modify-write* of `i` — it reads\n"
            "the prior value, adds $j, writes back.  When it appears inside a cmd-sub\n"
            "(`lappend r [incr i $j]`), the read is otherwise invisible to the outer\n"
            "word scanner, so the feeding `set i 0` looked dead (W220) and `i` looked\n"
            "unused (W211).  Fix: `command_sub_read_modify_write_names` in\n"
            "compiler/var_refs.py recovers incr/append/lappend targets nested in cmd-subs\n"
            "and treats them as reads of the prior version (commit cd98a579 et al.)."
        ),
        source=_dedent(
            """
            proc f {} {
                # incr inside the cmd-sub reads `i` (the prior value) — so the
                # feeding `set i 0` is alive, not a dead store.
                set i 0
                foreach j {1 2 3} { lappend r [incr i $j] }
                return $r
            }
            """
        ),
    ),
)


register(
    "FP-DS-02",
    _Entry(
        label="reads inside [expr {...}] command-sub recovered as real uses",
        proc="::f",
        vars=("w", "i"),
        show=("ssa", "dead", "unused"),
        notes=(
            "Tcl semantics: `incr i [expr {$w}]` evaluates the expr cmd-sub which reads\n"
            "$w at run time — so $w is genuinely *used*.  Pre-fix the expr body was opaque\n"
            "to the outer word scanner, so a `set w …` immediately before the incr looked\n"
            "dead / unused.  Fix (commit 16df8c4a): `statement_cmd_sub_read_names` walks\n"
            "IRAssignExpr/IRExprEval/IRReturn expr ASTs collecting variable reads under\n"
            "cmd-sub barriers so the feeding def is kept live."
        ),
        source=_dedent(
            """
            proc f {} {
                # $w is read inside the [expr {...}] cmd-sub — `set w 5` is NOT
                # a dead store, and `w` is NOT unused.
                set w 5
                set i 0
                incr i [expr {$w}]
                return $i
            }
            """
        ),
    ),
)


register(
    "FP-DS-03",
    _Entry(
        label="eval {literal-body} reads recovered (eval runs in caller scope)",
        proc="::f",
        vars=("x",),
        show=("ssa", "dead", "unused"),
        notes=(
            "Tcl semantics: `eval {puts $x}` runs the braced body in the *caller* scope —\n"
            "the $x read is a real read of the caller-local `x`.  Pre-fix the eval body\n"
            "was an opaque IRCall barrier, so a feeding `set x 1` looked dead / unused.\n"
            "Fix (commit 6f69c86b): `eval_body_read_names` in compiler/var_refs.py walks\n"
            "literal `eval {...}` and `namespace eval ns {...}` bodies recovering reads\n"
            "(including those nested inside `[expr {...}]` and `[set y ...]`)."
        ),
        source=_dedent(
            """
            proc f {} {
                # eval's braced body runs in the current scope; `$x` read here is
                # a real read of the local `x`.
                set x 1
                eval {puts $x}
            }
            """
        ),
    ),
)


register(
    "FP-DS-04",
    _Entry(
        label="traced variables excluded from dead-store / unused (soundness)",
        proc="::f",
        vars=("x",),
        show=("ssa", "dead", "unused"),
        notes=(
            "Tcl semantics: `trace add variable x write cb` (and the 8.4 form\n"
            "`trace variable x w cb`) install a write-trace callback on `x`.  Any\n"
            "subsequent `set x …` is *observable* via the callback — even if no later\n"
            "read appears in this proc.  So the write is NOT a dead store, and `x` is\n"
            "NOT unused.  Pre-fix the dead-store analysis ignored traces.  Fix\n"
            "(commit 6ced305d): collect `traced_var_names` from `trace add variable` /\n"
            "`trace variable` calls in compiler/var_refs.py; both _dead_store and\n"
            "_unused exempt them name-level."
        ),
        source=_dedent(
            """
            proc f {} {
                # The write is observable through the callback — must NOT fire
                # W220 (dead-store) or W211 (unused).
                trace add variable x write cb
                set x 1
            }
            """
        ),
    ),
)


register(
    "FP-DS-05",
    _Entry(
        label="CFGReturn read is a real use ($x kept live by `return $x`)",
        proc="::f",
        vars=("x",),
        show=("ssa", "dead", "unused"),
        notes=(
            "Tcl semantics: `return $x` reads `x` and propagates its value as the proc's\n"
            "return value.  The terminator-level read used to be invisible to the outer\n"
            "name-level recovery (CFGReturn carries a value expression but not a uses\n"
            "set on the last block-statement), so a `set x 1` immediately followed by\n"
            "`return $x` looked dead / unused.  Fix (return-read recovery): include\n"
            "CFGReturn value-expression reads when building the variable use-set;\n"
            "see compiler/var_refs.py terminator handling."
        ),
        source=_dedent(
            """
            proc f {} {
                # return $x reads $x — `set x 1` is NOT a dead store, `x` is NOT unused.
                set x 1
                return $x
            }
            """
        ),
    ),
)


register(
    "FP-DS-06",
    _Entry(
        label="array-element dead-store distinction: $a(k) write is not killed by $a(j) write",
        proc="::f",
        vars=(),
        show=("ssa", "dead", "unused"),
        notes=(
            "Phase 8G / 8D: with the ARRAY_ELEM Place model, distinct array-element\n"
            "writes are distinct memory locations, so `set a(k) 1; set a(j) 2; puts $a(k)`\n"
            "does NOT make `set a(k) 1` a dead store — `set a(j) 2` writes a different\n"
            "Place.  Pre-Phase-8 the analysis tracked whole-array kills, so the first\n"
            "write looked overwritten by the second.  Corpus: W220 −88, W211 −2, O109 −66.\n"
            "Sound because the 8E refinement (`dynamic` is an alias-target wildcard, not\n"
            "a *name* wildcard) keeps overlap suppress-only.\n\n"
            "Note: literal keys `k` / `j` are syntactically distinct, so the bench shows\n"
            "no spurious dead-store — exactly the pre-FP-DS-06 verdict that proves the\n"
            "Place model preserves the necessary disjointness."
        ),
        source=_dedent(
            """
            proc f {} {
                # k and j are distinct array element Places — set a(k) is NOT
                # killed by set a(j); the read of $a(k) makes the first write live.
                set a(k) 1
                set a(j) 2
                puts $a(k)
            }
            """
        ),
    ),
)


# PR 3 / SH family — shimmer (S100/S101/S102) determinations.


register(
    "FP-SH-01",
    _Entry(
        label="OVERDEFINED values do not trigger shimmer (conservative suppression)",
        proc="::top",
        vars=("x",),
        show=("ssa", "values"),
        notes=(
            "Shimmer (S100/S101/S102) reports a value flowing into an operator it\n"
            "wasn't created for — STRING into arithmetic, INT into string compare, etc.\n"
            "When the SCCP lattice resolves a value to OVERDEFINED (e.g. an unknown\n"
            "command return), the type is *unknown* — issuing a shimmer warning would\n"
            "be unsound.  The shimmer pass treats OVERDEFINED and UNKNOWN as exemptions\n"
            "(analyser/checks/_shimmer.py).  This locks the conservative behaviour in."
        ),
        source=_dedent(
            """
            # x has unknown type (cmd return) -> OVERDEFINED -> no shimmer warning.
            set x [unknownCmd]
            set y [expr {$x + 1}]
            return $y
            """
        ),
    ),
)


register(
    "FP-SH-02",
    _Entry(
        label="scope-alias declarations typed OVERDEFINED (not STRING) — kills shimmer FPs",
        proc="::f",
        vars=("v",),
        show=("ssa", "values"),
        notes=(
            "Pre-fix (commit adfc6d84): scope-alias declarations (`variable name`,\n"
            "`global name`, `upvar 1 src dst`) defaulted their declared local to\n"
            "TclType.STRING in the type lattice.  But an alias's intrep is determined\n"
            "by whatever the *target* is (which may be set externally), not by the\n"
            "alias-declaration spelling — STRING was an unsound guess that triggered\n"
            "spurious S100/S101 the first time the alias hit an arithmetic op.  Fix:\n"
            "type alias declarations as OVERDEFINED (truly unknown), exempting them from\n"
            "the shimmer pass like FP-SH-01.  Same principle as SCCP's `force-OVERDEFINED-\n"
            "for-escaping` rule in core_analyses.py."
        ),
        source=_dedent(
            """
            proc f {} {
                # `variable v` declares an alias — type is unknown (OVERDEFINED),
                # NOT STRING, so `expr {$v + 1}` must NOT fire S100.
                variable v
                return [expr {$v + 1}]
            }
            """
        ),
    ),
)


register(
    "FP-SH-03",
    _Entry(
        label="phi joins are hash-seed-independent (deterministic shimmer)",
        proc="::f",
        vars=("x",),
        show=("ssa", "values"),
        notes=(
            "Pre-fix (commit b08f2c47): SSA type-propagation joined types via a Python\n"
            "set iteration order at phi nodes, so the resulting joined type depended on\n"
            "PYTHONHASHSEED.  A loop-merged value could randomly come out STRING or\n"
            "INT, making shimmer warnings nondeterministic across runs.  Fix: sort\n"
            "phi-source type entries by a canonical key before reducing — the join is\n"
            "now stable.  The bench locks the verdict in by computing the lattice on a\n"
            "loop-merged variable (the historically flaky case)."
        ),
        source=_dedent(
            """
            proc f {n} {
                # x is joined at the loop header from two INT branches; the join
                # must come out INT every run (no flake) -> no S101.
                set x 0
                for {set i 0} {$i < $n} {incr i} {
                    if {$i > 5} { set x 1 } else { set x 2 }
                }
                return [expr {$x + 1}]
            }
            """
        ),
    ),
)


# PR 4 / OBJ family — object dispatch (W307/W308) + snit modelling.


register(
    "FP-OBJ-01",
    _Entry(
        label="snit self-references ($self/$type/$selfns/$win) — not stray non-literal commands",
        proc="::f",  # placeholder proc inside source; snit class-level
        vars=(),
        show=("ssa",),
        dialect="tcl8.6",
        notes=(
            "tclsh / snit semantics: inside a `snit::type` / `snit::widget` method body,\n"
            "`$self foo` dispatches *method* `foo` on the current object; `$type bar`\n"
            "dispatches a typemethod; `$selfns` is the per-instance namespace; `$win`\n"
            "is the widget's window path.  Dispatching on any of these is method dispatch,\n"
            "not the stray non-literal command word W307 was designed to catch.  Fix\n"
            "(snit modelling): `compiler.snit` collects the snit-reserved set and registers\n"
            "the type/widget body as a ClassDef; analyser/checks/_object_dispatch.py exempts\n"
            "the reserved names inside the type body.\n\n"
            "Bench note: snit bodies don't render through `::f`-style snapshots; this entry\n"
            "is locked in by the test pair (no per-line SSA evidence needed beyond the\n"
            "verdict that the diagnostic does/doesn't fire)."
        ),
        source=_dedent(
            """
            # Bench placeholder: snit modelling sits outside the per-proc snapshot.
            # The FP-OBJ-01 verdict is locked in by tests/test_fp_obj.py only.
            proc f {} { return ok }
            """
        ),
    ),
)


register(
    "FP-OBJ-02",
    _Entry(
        label="snit::widgetadaptor $hull dispatch — widgetadaptor delegation idiom",
        proc="::f",
        vars=(),
        show=("ssa",),
        notes=(
            "`snit::widgetadaptor` exposes the underlying widget through `$hull`;\n"
            "`$hull configure -bg red` is the canonical delegation pattern (tclsh +\n"
            "snit-verified).  The snit-reserved set includes `hull` so it joins\n"
            "`self`/`type`/`selfns`/`win` in being exempt from W307 inside the body.\n\n"
            "Locked in by tests/test_fp_obj.py; per-proc bench rendering doesn't fit."
        ),
        source=_dedent(
            """
            # See note in FP-OBJ-01 — snit modelling is class-level.
            proc f {} { return ok }
            """
        ),
    ),
)


register(
    "FP-OBJ-03",
    _Entry(
        label="snit component dispatch ($myexporter export ...) — instance-var method dispatch",
        proc="::f",
        vars=(),
        show=("ssa",),
        notes=(
            "`component myexporter` declares an instance-var holding a sub-object\n"
            "command.  Inside a snit method body, `$myexporter export $self $fmt`\n"
            "dispatches on a known object — same kind as `$self`/`$hull`, just user-\n"
            "declared.  Pre-fix the analyser couldn't distinguish `component`-declared\n"
            "vars from arbitrary instance vars and W307 fired for every `$var cmd`.\n"
            "Fix: the type-body inventory in `compiler.snit` collects `component` /\n"
            "`variable` / `option` / `typevariable` declarations into a per-type set;\n"
            "instance-var dispatches inside the body are exempt.\n\n"
            "Locked in by tests/test_fp_obj.py; class-level construct, no per-proc bench."
        ),
        source=_dedent(
            """
            proc f {} { return ok }
            """
        ),
    ),
)


register(
    "FP-OBJ-04",
    _Entry(
        label="namespaced-factory provenance: set t [::struct::tree] — object handle",
        proc="::f",
        vars=("t",),
        show=("ssa", "values"),
        notes=(
            "tcllib idiom: `::struct::tree` (with no args, or with a name arg) returns\n"
            "a command name for the new tree object; calling `$t walk root` dispatches\n"
            "to the object's method.  Pre-fix, the analyser saw `$t` as a non-literal\n"
            "command word and fired W307.  Fix (commit 880c3a15): provenance — a var\n"
            "assigned from a *namespaced* command substitution (`[::ns::factory …]` /\n"
            "`[ns::factory …]`) is tagged as an object handle; the dispatch is\n"
            "exempted.  Pure-bare unknown commands (`[foo bar]`) are NOT exempted —\n"
            "see FP-OBJ-XX (control test) in tests/test_fp_obj.py.  This is analyser-\n"
            "only provenance, no type-lattice change, so no shimmer collateral."
        ),
        source=_dedent(
            """
            proc f {} {
                # ::struct::tree is a namespaced factory; $t is an object handle.
                set t [::struct::tree mytree]
                $t walk root
            }
            """
        ),
    ),
)


register(
    "FP-OBJ-05",
    _Entry(
        label="snit instance dispatch (set o [Foo create %AUTO%]; $o m) — typed OBJECT",
        proc="::use",
        vars=("a",),
        show=("ssa", "values"),
        notes=(
            "Pre-fix the analyser couldn't see that `Foo create %AUTO%` returns an\n"
            "instance of the locally-defined snit type, so dispatching on the result\n"
            "(`$a bump`) fired W307.  Fix: local snit-type definitions are registered\n"
            "in `compilation_unit.snit_types`; a var initialised from a call to that\n"
            "type's create-form (or the create-shorthand `Foo %AUTO%`) is typed OBJECT\n"
            "and its dispatch is exempted.  Also exempts W308 method validation since\n"
            "snit method dispatch goes through delegation/hull/options and isn't\n"
            "soundly resolvable to the declared set."
        ),
        source=_dedent(
            """
            snit::type ::Counter { method bump {} { return 1 } }
            proc use {} {
                # `Counter create %AUTO%` returns a snit instance; $a bump is
                # method dispatch, not a stray non-literal command.
                set a [Counter create %AUTO%]
                $a bump
            }
            """
        ),
    ),
)


register(
    "FP-OBJ-06",
    _Entry(
        label="snit private proc body is analysed (not silently dropped)",
        proc="::f",
        vars=(),
        show=("ssa",),
        notes=(
            "A `proc` declared inside a `snit::type` body is a type-private proc — its\n"
            "body must still be analysed (tclsh + snit-verified: the proc runs when\n"
            "called from the type's methods).  Pre-fix the analyser stopped at the\n"
            "type body, dropping the inner proc's body and any genuine diagnostics it\n"
            "would have raised.  Fix: snit::type/widget bodies register inner procs\n"
            "with the analyser pipeline so their bodies receive the usual treatment.\n\n"
            "Locked in by tests/test_fp_obj.py — verifies W216 fires inside the proc body."
        ),
        source=_dedent(
            """
            # Class-level body; per-proc snapshot doesn't fit.
            proc f {} { return ok }
            """
        ),
    ),
)


# PR 5 / RCH family — reachability (O107) determinations.


register(
    "FP-RCH-01",
    _Entry(
        label="while 1 { break }: break-after is reachable (not O107 dead code)",
        proc="::f",
        vars=(),
        show=("ssa",),
        notes=(
            "Pre-fix: `break`/`continue` were modelled as jump statements, not CFG\n"
            "edges, so the loop-exit block was reachable only via the loop header's\n"
            "exit edge — which SCCP prunes as dead when the condition is `1`.  Code\n"
            "AFTER `while 1 { … break … }` was wrongly flagged O107 (unreachable\n"
            "dead code), and DCE was *unsound* — it could delete still-reachable\n"
            "statements.  Fix: SCCP now feeds the `break → loop-exit` edge into the\n"
            "reachability worklist, so the post-loop block stays reachable."
        ),
        source=_dedent(
            """
            proc f {c} {
                # while 1 with a conditional `break` -> `puts after` IS reachable.
                while 1 { if {$c} break }
                puts after
            }
            """
        ),
    ),
)


register(
    "FP-RCH-02",
    _Entry(
        label="try handler body is reachable (analysis-only exception edges)",
        proc="::f",
        vars=(),
        show=("ssa",),
        notes=(
            "Pre-fix: `try { … } on error {e opts} { … }` lowered without a CFG\n"
            "predecessor edge into the handler block — the handler body was a CFG\n"
            "island, so every statement in it fired O107.  Fix: SSA construction adds\n"
            "*analysis-only* exception edges from the try body into each handler;\n"
            "codegen ignores them so default bytecode stays tclsh-identical, but the\n"
            "analyser sees a reachable handler.  See compiler/ssa.py exception-edge\n"
            "construction."
        ),
        source=_dedent(
            """
            proc f {} {
                # `on error` handler body is reachable; no O107 on `set y 1`.
                try {
                    set x [doThing]
                } on error {e opts} {
                    set y 1
                    puts $y
                }
            }
            """
        ),
    ),
)


register(
    "FP-RCH-03",
    _Entry(
        label="on ok inherits body-defined SSA versions (no W210 on body-set var)",
        proc="::f",
        vars=("vdata",),
        show=("ssa", "rbs"),
        notes=(
            "`on ok` runs *after* the try body completes normally, so any var set in\n"
            "the body is defined when the handler runs (tclsh-verified).  Pre-fix the\n"
            "handler block didn't inherit body SSA versions — it saw `vdata#0`\n"
            "(sentinel-before-any-def) and fired W210 'read before set'.  Fix: the\n"
            "ok-path exception edge feeds the body's last-version map into the handler\n"
            "phi inputs, matching the natural sequential control flow."
        ),
        source=_dedent(
            """
            proc f {} {
                # `on ok` runs after the body completes; $vdata IS defined.
                try {
                    set vdata [getData]
                } on ok {} {
                    return $vdata
                }
            }
            """
        ),
    ),
)


register(
    "FP-RCH-04",
    _Entry(
        label="genuine infinite-loop (no break) -> code after IS unreachable (TP control)",
        proc="::f",
        vars=(),
        show=("ssa",),
        notes=(
            "TP / control test: an infinite loop with NO `break` (or `return` /\n"
            "uncaught exception) really does make the post-loop statements unreachable.\n"
            "O107 must still fire here — proves FP-RCH-01's fix isn't blanket-\n"
            "suppressing all post-loop reachability claims."
        ),
        source=_dedent(
            """
            proc f {} {
                # No break / return -> `puts after` IS dead code.
                while 1 { puts x }
                puts after
            }
            """
        ),
    ),
)


# PR 6 / INJ family — injection / style (W101/W105/W301/T102) determinations.


register(
    "FP-INJ-01",
    _Entry(
        label="uplevel 1 $body (bare var) is the safe idiom — NOT W301",
        proc="::f",
        vars=("body",),
        show=("ssa",),
        notes=(
            "Tcl semantics: `uplevel 1 $body` evaluates `$body` once in the target\n"
            "frame; the value is the script source.  Braces would block the variable\n"
            "expansion (`uplevel 1 {$body}` evaluates the literal text `$body`, not\n"
            "the variable's contents) — so W301's 'use braces' advice is wrong for\n"
            "this idiom.  Fix: W301 recognises the `$single_var` form as the safe\n"
            'case; only quoted-interpolation / multi-arg-concat forms (`"$cmd $arg"`)\n'
            "are flagged.  See analyser/checks/_uplevel.py."
        ),
        source=_dedent(
            """
            proc f {body} {
                # The canonical `uplevel 1 $body` pattern — must NOT fire W301.
                uplevel 1 $body
            }
            """
        ),
    ),
)


register(
    "FP-INJ-02",
    _Entry(
        label="eval [list ...] is safe — list-canonical form, NOT W101",
        proc="::top",
        vars=(),
        show=("ssa",),
        notes=(
            "`eval [list set $varname $value]` is the canonical-safe form: `[list …]`\n"
            "produces a list whose elements survive `eval`'s concatenation/re-parse\n"
            "without double substitution.  W101 (eval injection) deliberately exempts\n"
            "this form — the registry classifies `list` / `linsert` / `split` /\n"
            "`lreplace` / etc. as list-returning canonical-safe commands.  See\n"
            "analyser/checks/_eval.py."
        ),
        source=_dedent(
            """
            # Canonical safe form — eval of a list-returning cmd-sub.  No W101.
            eval [list set $varname $value]
            """
        ),
    ),
)


register(
    "FP-INJ-03",
    _Entry(
        label="T102 suppression: HTTP::uri PATH_PREFIXED -> no option injection",
        proc="::top",
        vars=(),
        show=("ssa", "values"),
        dialect="irules",
        notes=(
            "iRules semantics: `HTTP::uri` and `HTTP::path` return strings that\n"
            "*always* begin with `/` (path-anchored) — there's no way an attacker can\n"
            "make them start with `-`, so feeding them to a command that takes an\n"
            "option terminator (`regexp $uri …`) cannot cause option injection.  Fix:\n"
            "the taint pass tags HTTP::uri / HTTP::path values with the PATH_PREFIXED\n"
            "colour; T102 (option injection) is suppressed for any propagated value\n"
            "carrying that colour.  See compiler/taint/_sinks.py:_check_t102."
        ),
        source=_dedent(
            """
            set uri [HTTP::uri]
            regexp $uri test
            """
        ),
    ),
)


register(
    "FP-INJ-04",
    _Entry(
        label="T102 TP control: literal '-' prefix [HTTP::path] still warns",
        proc="::top",
        vars=(),
        show=("ssa", "values"),
        dialect="irules",
        notes=(
            "TP / control: prepending a fixed `-` literal to an HTTP::path value\n"
            "*does* produce an option-like string; the path-prefix safety from\n"
            "FP-INJ-03 doesn't apply when the attacker-controlled value is concatenated\n"
            "after a fixed `-`.  T102 must still fire.  Locks in the suppression's\n"
            "lower bound — it doesn't blanket-exempt every HTTP-derived value."
        ),
        source=_dedent(
            """
            set foo "-[HTTP::path]"
            regexp $foo test
            """
        ),
    ),
)


register(
    "FP-INJ-05",
    _Entry(
        label='eval "$cmd $x" -> W101 with code-action rewrite to eval [list ...]',
        proc="::top",
        vars=(),
        show=("ssa",),
        notes=(
            'TP: `eval "$cmd $x"` performs *double substitution* — every embedded\n'
            "`$var` and `[cmd]` is substituted by eval's re-parse on top of the outer\n"
            "substitution, which is the canonical Tcl injection vulnerability.  W101\n"
            "fires; the LSP code-action rewrites the call to the safe form `eval\n"
            "[list $cmd $x]` so `[list …]` quoting prevents double substitution.\n"
            "Locked in by tests/test_fp_inj.py — verifies both the diagnostic fires\n"
            "*and* the code-action produces the expected replacement text."
        ),
        source=_dedent(
            """
            # Top-level. eval of a non-list cmd-sub -> W101 + quick-fix.
            set x foo
            eval "process $x"
            """
        ),
    ),
)


# PR 7 / BND family — bounds / intervals (W230/W231/W232/W233).


register(
    "FP-BND-01",
    _Entry(
        label="W231 lset dynamic out-of-range loop index ($j > length) fires",
        proc="::f",
        vars=("j",),
        show=("ssa", "values", "dead"),
        notes=(
            "Phase-3 interval domain: a loop variable `j ∈ [4, 8]` against `set l\n"
            "{a b c}` (length 3) is provably > length on every iteration — tclsh\n"
            'errors with `index "4" out of range`.  Pre-Phase-3 the bounds check was\n'
            "purely literal-arg (FP-NAB-01); now it consults the interval lattice for\n"
            "dynamic indices too.  See compiler/interval_bounds.py and\n"
            "analyser/_analyser/_diag_interval_bounds.py."
        ),
        source=_dedent(
            """
            proc f {v} {
                # j is bounded [4, 8]; list length 3 -> every iteration is OOR.
                set l {a b c}
                for {set j 4} {$j < 9} {incr j} { lset l $j $v }
            }
            """
        ),
    ),
)


register(
    "FP-BND-02",
    _Entry(
        label="W231 dynamic append-slot ($j == length) IS silent (FP guard)",
        proc="::f",
        vars=("j",),
        show=("ssa", "values"),
        notes=(
            "FP guard for the W231 dynamic check: the dynamic-index path must mirror\n"
            "the literal-index path's `> length` (not `>= length`) comparison so the\n"
            "interval-tracked append slot stays silent.  Pre-fix the dynamic check\n"
            "used `>=` and fired W231 for `set j 3; lset l $j $v` on a 3-element list\n"
            "— which is the legal append slot (FP-NAB-01).  Fix: interval_bounds.py\n"
            "uses the same strict comparator."
        ),
        source=_dedent(
            """
            proc f {v} {
                # j == length -> APPEND slot; must NOT fire W231.
                set l {a b c}
                set j 3
                lset l $j $v
            }
            """
        ),
    ),
)


register(
    "FP-BND-03",
    _Entry(
        label="W232 string index past end ($i >= length) fires (string smell)",
        proc="::f",
        vars=("i",),
        show=("ssa", "values"),
        notes=(
            'Phase-3: `string index $s $i` returns `""` silently when `$i` is out of\n'
            "range (tclsh-verified) — same severity tier as W230 (lindex smell), NOT\n"
            "W231 (lset error).  The interval domain tracks string-length per\n"
            'SSA-version (compiler/interval_bounds.py) so a constant-`set s "hello"`\n'
            "+ constant-`set i 10` is provably OOR."
        ),
        source=_dedent(
            """
            proc f {} {
                # i (10) > string length (5) -> tclsh returns ""; W232 smell.
                set s "hello"
                set i 10
                return [string index $s $i]
            }
            """
        ),
    ),
)


register(
    "FP-BND-04",
    _Entry(
        label="W233 division by a provably-zero divisor (constant $d=0) fires",
        proc="::f",
        vars=("d",),
        show=("ssa", "values"),
        notes=(
            "Phase-3: `expr {10 / $d}` with `set d 0` is a tclsh divide-by-zero\n"
            "runtime error.  The interval domain proves `d == 0` (CONST lattice)\n"
            "and the deep-finding pass emits W233.  Locked in here; FP-BND-05 is the\n"
            "matching FP-guard for short-circuited dead arms."
        ),
        source=_dedent(
            """
            proc f {} {
                # $d == 0 (CONST) -> tclsh divide-by-zero; W233 fires.
                set d 0
                return [expr {10 / $d}]
            }
            """
        ),
    ),
)


register(
    "FP-BND-05",
    _Entry(
        label="W233 FP guard: dead ternary arm / short-circuit `1 || 1/0` is silent",
        proc="::f",
        vars=(),
        show=("ssa", "values"),
        notes=(
            "Tcl `expr` is lazy: the dead arm of `?:` and the short-circuited operand\n"
            "of `&&`/`||` never execute, so a `1/0` in those positions is NOT a runtime\n"
            "error (tclsh 9.0.3-verified: `expr {0 ? 1/0 : 7}` returns 7, `expr {1 ||\n"
            "1/0}` returns 1).  W233 must respect short-circuit semantics in the expr\n"
            "AST walker; firing here would be a FP.  See compiler/intervals.py\n"
            "expression eval + the W233 deep-finding entrypoint."
        ),
        source=_dedent(
            """
            proc f {} {
                # `0 ? 1/0 : 7` -> dead arm; tclsh returns 7; W233 must NOT fire.
                return [expr {0 ? 1/0 : 7}]
            }
            """
        ),
    ),
)


def _render(fp_id: str) -> str:
    entry = ENTRIES[fp_id]
    snap = _pick(entry.source, entry.proc, dialect=entry.dialect)
    body = render_evidence(snap, vars_of_interest=entry.vars, show=entry.show)
    header = f"--- {fp_id}: {entry.label}\nregen: python -m bench.fp_snippets --id {fp_id}\n"
    return header + body


def main(argv: Sequence[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--id", help="render one FP-ID")
    p.add_argument("--list", action="store_true", help="list registered FP-IDs")
    args = p.parse_args(argv)
    if args.list:
        for fp_id, entry in ENTRIES.items():
            print(f"{fp_id}  {entry.label}")
        return 0
    if args.id:
        if args.id not in ENTRIES:
            print(f"unknown FP-ID {args.id!r}; --list to enumerate", file=sys.stderr)
            return 2
        print(_render(args.id))
        return 0
    p.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
