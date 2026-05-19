# Zig WASM runtime roadmap — phases 4 (residual), 5, 6

Companion to the master plan at
`tmp/wasm-perf-report/REPORT.md` and the after-action snapshot at
`docs/perf/wasm-tcl9-parity/09-after-action.md`.  This doc captures
the **work that didn't ship in the unattended Phase 0–4 push** —
why each item was deferred, the concrete first step for picking it
up, and the acceptance gate.

The reader should already be familiar with the layout in
[`AGENTS.md`](../../../AGENTS.md) (Zig runtime layering),
[`compiler-architecture.md`](../compiler-architecture.md), and the
existing introspection / namespace-tree design notes alongside this
file.

## Quick map

| Item | Status | First file to touch |
|---|---|---|
| 4.4 — `ConstraintInitializer must be complete script` | reproducer needed | `runtime/zig/cmds/tcl_cmd_info.zig::info_complete` |
| 4.5 — regexp `-indices` / `-all` / `-inline` / matchVar | scaffolding shipped, capture is no-op | `runtime/zig/valtypes/tcl_regex.zig::eval_regexp_cmd` |
| 5.1 — Specialise `string length` | not started | new `cmds/string_.py` lowering hook |
| 5.2 — Specialise `lindex` of constant index | not started | new `lowering_hooks/_list.py` |
| 5.3 — Specialise `dict get` of constant key | not started | `runtime/zig/valtypes/tcl_dict.zig` + new lowering hook |
| 5.4 — Tail-call codegen conversion | detection wired, codegen disabled | `core/optimiser/_tail_call.py` → emit IRTailCall |
| 5.5 — Switch options lowering hook | runtime side done, lowering still routes through runtime | `compiler/lowering.py::_lower_switch` |
| 5.6 — In-flight `expr` const-folding | passes exist pre-codegen | `compiler/codegen/wasm/_emitter/_expressions.py` |
| 6.1 — TclOO scaffolding | not started | new module under `runtime/zig/cmds/oo_*.zig` |
| 6.2 — Coroutines + NRE | not started | architecture decision needed first |
| 6.3 — Parser strict mode | not started | `compiler/parsing/expr_parser.py` |

---

## Residual Phase 4 work

### 4.4 — `ConstraintInitializer must be complete script`

**Symptom:** `parseExpr.test` and `dict.test` trap in tcltest's
`tcltest::ConstraintInitializer` proc with the literal error
``ConstraintInitializer must be complete script``.  That message is
the proc's own `return -code error` after `[info complete $script]`
returns 0.

**Status:** diagnosed.  Blocked on Phase 1.3 (allocator `(ptr, len)`
lifetime audit) — *not* an `info_complete` walker bug.

**Diagnostic finding (Phase 4.4.A, this branch):**
The instrumented `info_complete` (temporary stderr dump of every
input it received during a `parseExpr.test` run, removed in the
same commit) shows the script TclObj reaching the walker has the
*correct length* but *corrupted bytes*.  Sample captures from the
failing run:

```
[ic] len=43 bytes='                 _pla        \x907        '
[ic] len=46 bytes='    `\x90)H\xb6     _platform(pl    rm) windows'
[ic] len=23 bytes='te    nstraint uni    y'
[ic] len=51 bytes='expr {[tes    straint unix     [testConstr     pc]}'
```

The lengths line up with the expected scripts
(`string equal $::tcl_platform(platform) windows` ≈ 46 chars,
`testConstraint unixOnly` ≈ 23, `expr {[testConstraint unix] || [testConstraint pc]}` ≈ 51) but the bytes have been
partially overwritten — looks like a buffer that was freed back to
the size-class free-list, then reissued and partially rewritten,
while the original TclObj's `OBJ_STR_PTR` still points there.

The walker is correctly returning 0 ("incomplete") on these byte
sequences (they have unbalanced braces or stray nulls).  The bug
is upstream: the proc parameter's underlying buffer was freed and
reused before the proc body finished reading from it.

**Why this is Phase 1.3 work:**
This is the same class of bug that produces the ``unknown command:
<garbled>`` traps in 9 other tcltest files (see
`tmp/wasm-perf-report/REPORT.md`).  A `(ptr, len)` reference is
held across a call that transitively reaches `obj_release` →
size-class free-list → `obj_alloc` re-issue.  Phase 1.3's
deliverable explicitly covers this audit.

**Workaround attempts that did NOT fix it** (so future work
doesn't re-tread):

- Outer-brace stripping in `execute_parsed_command` for `tok.braced`
  words — the bytes the parser produces are correct content
  bytes; the corruption happens later.
- Proc-side parameter copy via `obj_new_string_copy` — defers but
  doesn't eliminate the lifetime bug; the copied buffer can also
  be freed and reissued.

**Real fix path (when Phase 1.3 lands):**
Audit every `obj_release` site that runs while a parser-borrowed
TclObj is still in a frame.  Either retain the obj at parameter
binding time (refcount += 1 in `eval_proc_call_bucket` line 1160
for each `frames.local_set(param_name, words[arg_idx])`) and
release on frame_pop, OR copy the bytes into a frame-local buffer
that lives until frame_pop.

**Acceptance (when Phase 1.3 lands):** `parseExpr.test` and
`dict.test` reach a tcltest summary line.  The 9 garbled-command
trap files in 08-tcltest-suites.md should also clear in the same
commit.

### 4.5 — regexp result-shape modes (capture / -indices / -inline / -all)

**Status as of this branch:** option *parsing* is complete; result
*shaping* for `-indices`, `-inline`, `-all`, and capture-var
assignment is wired but currently returns empty/zero because
`run_match_cap` returns `false` from inside `eval_regexp_cmd`'s
loop on patterns that match correctly via `run_match`.

**Critical clue:** the same `run_match_cap` invocation succeeds
when called from `do_regsub` (which is exercised by tcltest's
`regsub` calls and produces correct substitutions).  So the regex
engine is fine; something in the eval_regexp_cmd call frame
breaks.

**Repro plan:**

1. Reduce `eval_regexp_cmd` to just the compile + a single
   `run_match_cap` call (drop the loop, drop `-all`, drop inline
   buffer building) and verify that returns true.
2. If that fails, the bug is in argument plumbing — most likely
   `pat_u`/`sub_u` aliasing via the bump allocator overlapping
   with the per-match `pmatch_buf`.  Move the `pmatch_buf` alloc
   *before* `decode_utf8(sub_s, …)` so the sub_u tail can't be
   stomped.
3. If step 1 succeeds, the bug is in the loop's reuse of `re_ptr`
   across calls.  Try recompiling per iteration as a sanity check.

**Acceptance:**
- `regexp -inline {a+} aaa` returns ``aaa``
- `regexp {(\w+)\s+(\w+)} "foo bar" -> a b` sets a=foo, b=bar
- `lseq.test`, `lrepeat.test`, `reg.test`, `switch.test` reach a
  summary line.

**Switch -matchvar/-indexvar:** depends on the same fix landing.
The runtime switch dispatch currently doesn't accept these
options; the lowering hook in `compiler/codegen/wasm/_emitter/
_control_flow.py::_emit_switch` would need to detect them and
call into a new `eval_switch_cmd` that uses the same
`run_match_cap` plumbing.

---

## Phase 5 — Compiler-side specialisation

These items move work from the runtime dispatch path into directly
emitted WASM instructions.  Each is risky because the
compiled-vs-interpreted divergence already produces subtle bugs
(see Phase 4.1 var_resolve fix for an example), so all of them need
a focused round of integration tests before landing.

### 5.1 — Specialise `string length`

**Goal:** turn `string length $x` from a runtime `tcl_cmd_string`
dispatch into an inline i32 load from the TclObj header at
`OBJ_STR_LEN`.

**Why deferred:** the runtime entry point already exists
(`string_length` in `valtypes/tcl_string.zig:81`); the compiler-
side hook for the value-context path needs to be added in
`compiler/codegen/wasm/_emitter/cmds/` (no `string_.py`
exists today — the dispatch goes through the generic runtime
path).  Estimated 1 day; the gain is roughly 200 ns → 30 ns per
call, or ~6× on tight string-length loops (string-template engines,
log emitters, validators).

**First step:** copy the structure of
`cmds/format_.py` (which has its own dispatch for the variadic
case) and add an emit hook that:

1. Detects `string length <single-arg>` where the arg is a literal
   or a single var read.
2. Emits the var/literal resolution, then a direct i32 load from
   `OBJ_STR_LEN` on the resulting TclObj address.
3. Falls back to the runtime call for variadic / sub-expr cases.

**Acceptance:** microbench `string length $s` × 100,000
≤ 30 ns/op (vs ~200 ns today).

### 5.2 — Specialise `lindex` of constant index

**Goal:** `lindex $L 0` and `lindex $L 1` become a direct call to
the runtime's `list_element_at` with a precomputed integer index.
Today the codegen passes an i32 TclObj wrapping the index, which
requires a heap allocation and a second integer parse on the
runtime side.

**First step:** add a `lowering_hooks/_list.py` that detects
`lindex <var> <small-literal>` and emits an `IRCall` flavour with
the index baked in.  The runtime entry can stay the same;
`list_element_at` already accepts an integer index.

**Acceptance:** `lindex $L 0` × 100,000 ≤ 60 ns/op.

### 5.3 — Specialise `dict get` of constant key

**Goal:** when the dict key is a compile-time literal, hash it at
compile time and pass the precomputed hash to the runtime so it
skips the FNV1a pass at runtime.

**Why bigger:** this needs a new runtime export
(`dict_get_prehashed`) and a coordinated change in
`runtime/zig/valtypes/tcl_dict.zig` to accept a
``(key_ptr, key_len, hash)`` tuple.  Implementation is
straightforward but touches the dict layout module.

**First step:** add the prehashed entry point to `tcl_dict.zig`,
matching the existing `dict_get` shape but skipping the hash
computation.  Then wire up a Python-side hook that emits the
prehashed call when the key is a literal.

**Acceptance:** `dict get $D constkey` per-op ≤ 80 ns/op.

### 5.4 — Tail-call codegen conversion

**Goal:** when `core/optimiser/_tail_call.py` (which already
detects O121 sites) flags a self-recursive tail call, the codegen
emits a `loop` + parameter rebinding instead of a `call`.  Saves
the per-call frame_push/pop and removes the stack-growth ceiling
on iterative recursive procs.

**Why deferred:** the detection result is currently diagnostic-
only.  Wiring it through requires the IR proc node to carry a
`tail_call_sites: list[IRStmtId]` field, the codegen to look at
that field and emit a different shape for the tail position, and
correctness coverage for `[upvar]` / `[info level]` interactions.

**First step:** read `_tail_call.py:O121–O123` and decide whether
to surface the detection result via a new IR node (`IRTailCall`)
or via metadata on the existing `IRCall`.  The IR node form is
cleaner because the optimiser can transform whole-proc bodies
into a single loop.

**Acceptance:** factorial(1,000,000) iterative-style runs without
a stack overflow; synthetic test asserts the wasm output for the
recursive proc body contains zero `call` instructions in the
recursion path.

### 5.5 — Switch options lowering hook

**Goal:** `switch -exact $x { p1 b1 p2 b2 ... }` with constant
patterns becomes a direct branch tree in WASM, skipping the
runtime dispatch entirely.  Currently the codegen passes the
whole switch through `IRSwitch` which evaluates against the
runtime.

**First step:** locate the existing `_lower_switch` in
`compiler/lowering.py` and detect the all-constant-pattern
case.  Add a new IR node (or a flag on `IRSwitch`) so the
codegen can pick the branch-tree form.  `-glob` / `-regexp` /
`-matchvar` / `-indexvar` keep the runtime form.

**Acceptance:** hand-rolled `switch $i { 1 {} 2 {} ... }` ×
100,000 with constant patterns shows ≥ 3× speed-up.

### 5.6 — In-flight `expr` const-folding

**Goal:** `set y [expr $x + 1]` where `$x` resolves to a literal
folds to `set y <constant>` at codegen time.

**Why deferred:** the SCCP / GVN / type-lattice passes already do
this *pre-codegen*, but the codegen-time path emits an
`IRExprEval` call regardless because the constant info doesn't
flow forward.  Needs the const-prop result to be attached to the
IR expression node so codegen can see it.

**First step:** in `core/optimiser/_propagation.py`, attach the
folded constant to the IR node (via a new `.const_value` field
or similar) and have `_emit_expr` short-circuit when present.

**Acceptance:** synthetic snippet `set x 5; set y [expr $x + 1]`
emits two literal stores, no expression-eval call in the wasm.

---

## Phase 6 — Long-tail features

### 6.1 — TclOO scaffolding

**What's required:** class table + method dispatch + `oo::class
create` + `oo::define` + `oo::object` + per-class private/public
visibility + inheritance + `next` chaining.  This is multi-day
work and should be its own focused PR series.

**Where to start:**

1. Read `tcl9.0.3/generic/tclOO.c` and the public `tclOO.h`
   header.
2. Decide layering: pure-Zig in a new `runtime/zig/cmds/oo/`
   module tree, or stub-into-Tcl-implementation that calls
   into the eval-fallback for everything until the C-shape is
   reproduced.  Recommendation: pure-Zig; the runtime already
   has command-table and namespace machinery.
3. Implement minimal `oo::object create foo` first.  Add
   `method` next.  Inheritance + `next` last.

**Acceptance:** `oo.test` reaches a tcltest summary line and at
least the basic class/method dispatch tests pass.

**Files affected:**

- `runtime/zig/cmds/oo_class.zig` (new)
- `runtime/zig/cmds/oo_define.zig` (new)
- `runtime/zig/dispatch/tcl_cmd_table.zig` (registration)
- `compiler/registry/tcl/oo_*.py` (specs already exist)

### 6.2 — Coroutines + NRE

**What's required:** `coroutine` / `yield` / `yieldto` /
`tailcall` machinery.  WASM doesn't have native stackful
coroutines, so the implementation has to either:

- Pass the runtime through Binaryen's `asyncify` pass — easy to
  bolt on but adds binary size and hurts hot paths.
- Implement explicit stack-spilling per coroutine, mirroring
  Tcl's NRE (non-recursive engine) state-machine model — more
  code but cleaner perf.

**Recommendation:** explicit NRE.  Starts with
`coroutine` as a thin wrapper over the existing eval loop, with
a state save/restore around `yield`.  No event loop — `yield`
hands control back to the calling proc immediately, similar to
Tcl 8.6's first cut.

**Acceptance:** `coroutine.test`, `nre.test`, `tailcall.test`
each reach a tcltest summary line; `yield` + `coroutine` round-
trip preserves variable state across the suspend.

### 6.3 — Parser strict mode

**Symptom:** the WASM runtime accepts unbraced expressions
(`if $a {…}` with `$a` containing a non-boolean) where Tcl 9
rejects them as `invalid bareword`.  Sample 5 in `samples/tcl/`
shows the divergence.

**What's required:**

- `compiler/parsing/expr_parser.py` — promote bareword detection
  from warning to error in `expr` context.
- `compiler/parsing/lexer.py` — stricter unbalanced-quote handling
  for unbraced word forms.
- An audit pass against `tcl9.0.3/generic/tclParse.c` to find
  every case where our parser is more permissive than reference.

**Acceptance:** sample 5's stdout matches `tclsh`; the in-scope
tcltest sweep shows no parse-related regressions.

---

## Coordination notes

- All Phase 5 items depend on the **frame-aware var resolution** fix
  shipped in Phase 4.1.  Without it, escape-elided procs that need
  to fall back to the interpreter for any reason can't see their
  caller's vars correctly.
- Phase 5.4 (tail-call codegen) and 5.5 (switch lowering) both
  need IR-shape changes that should be planned together — they
  share the "specialised IR node from optimiser" pattern.
- Phase 6.1 (TclOO) blocks four currently-trapping tcltest files
  (`oo`, `ooNext2`, `ooProp`, `ooUtil`).  Worth scheduling early
  in any push to drive the in-scope sweep pass-rate up.
- Phase 6.2 (coroutines) blocks three more (`coroutine`, `nre`,
  `tailcall`) but is the hardest engineering work in the backlog.

## Verification helpers

`scripts/dev/run_tcl9_tcltest_sweep.py` is the source of truth for
"did this change unblock a file?".
Each Phase 5/6 PR should:

1. Run the sweep before and after.
2. Diff against `tests/baselines/tcl9_tcltest_baseline.json`.
3. Fail the build if any previously-`pass` or -`partial` file
   regresses to `trap`.

`scripts/dev/perf_microbench.py` and `tests/baselines/wasm_microbench_baseline.json`
do the same for per-op cost.  Phase 5 acceptance numbers above are
specific microbench rows; gate the PR on hitting them.

## Where this doc lives

This file is `docs/design/runtime/zig-runtime-roadmap.md`.  Sibling
design notes:

- [`child-interp.md`](child-interp.md)
- [`command-introspection.md`](command-introspection.md)
- [`namespace-tree.md`](namespace-tree.md)
- [`rename-alias.md`](rename-alias.md)

Update this roadmap as items move from "deferred" to "shipped"
or as new sub-plans emerge from sweep evidence.
