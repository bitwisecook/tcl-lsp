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
