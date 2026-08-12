# Optimiser duplication / half-migration audit — `rust/tcl-compiler/src`

Branch `claude/legacy-code-duplication-audit-edm0bo` (based on `rust`, tip `0f97d98`).

**Note on the exemplar.** The PR #1371 shape is *still live on this branch*:
`compiler_checks.rs:355` calls `find_redundancies_for_function` (the modern
`GvnLegalitySource::Common` semantic-sidecar path, `gvn.rs:1419-1432`), while
`compiler_checks.rs:358` and `:361` call `find_partial_redundancies` /
`find_loop_invariants`, which both run on `GvnLegalitySource::Legacy` through
`statement_occurrences` / `statement_writes_state` (`gvn.rs:994-1011`, `:937-951`)
and therefore on the `is_pure_command` command-string classifier. I verified it
but do **not** re-report it — it is the given exemplar. F3 below is a *different*
defect inside those same two functions.

Every finding below was verified by reading both sides **and** by running the
built `tcl` CLI (`tcl opt` / `tcl diag` / `tcl explore --json`) on a reproducer.

---

## F1: Four optimiser rewrites ignore the dynamic-name barrier that dead-store elimination and SCCP both consult, and miscompile any function containing `set $name …`

**Confidence:** high
**Category:** soundness-asymmetry

**The modern path:** `crate::dynamic_names::DynamicNameBarrier`
(`dynamic_names.rs:143-155` — typed `{writes, destroys, reads}` flags) is built
per function by `dynamic_name_barrier` (`dynamic_names.rs:404`) and stored on
every `FunctionUnit` (`compilation_unit.rs:303`, populated at
`compilation_unit.rs:630`). Its documented contract is "after a dynamic write,
**any** name may be defined; after a dynamic read, **any** store may have been
observed". Consumers that respect it: the O109/O126/O108 elimination pass, which
abstains outright — `elimination.rs:446` `if fu.dynamic_names.reads { return
HashSet::new(); }`; SCCP's existence-branch folding (`sccp.rs:1152`, `:1176`,
`:1198`); and the analyser's dataflow diagnostics
(`analyser/diagnostics/dataflow.rs:282`, `:561`, `:1038`).

**The legacy/duplicate path:** four sibling passes decide the same "can an
unnamed variable access reach this variable?" question by hand, from IR text,
and never look at `fu.dynamic_names`:

- **O104/O130 chain fold** — `chain_fold.rs:73` (`run`), `:96-104`
  (`protected_vars`, which only unions `analyse_var_observability` +
  `cross_event_vars`). Worse, `classify_write` (`chain_fold.rs:241-252`)
  normalises the *dynamic* target word `$name` to the plain name `name`, so
  `chain_fold.rs:345` (`Some(other) if write_var(&other) != var => { j += 1 }`)
  walks the accumulation chain straight past `set $name …`.
- **O125 code sinking** — `code_sinking.rs:57-62` (`run`) plus
  `statement_uses_var` (`code_sinking.rs:579-584`), whose only "does anything
  else read this?" evidence is a textual `$var` scan; it even returns `false`
  for `Statement::Barrier` (`code_sinking.rs:584`).
- **O102 load forwarding** — `propagation.rs:223-241`; its doc comment
  (`propagation.rs:199-222`) enumerates its guards as
  `is_externally_mutable` + `has_intervening_barrier` and explicitly argues that
  "a plain proc-local variable … cannot be touched by an intervening call to any
  other proc". A dynamic `set $name …` in the *same* frame is exactly the case
  that argument misses.
- **O119 multi-set packing** — `pattern_recognition.rs:122-153`.

**Why it matters:** three demonstrated miscompiles on the current build.

1. `proc f {name} { set acc {}; lappend acc a; set $name zzz; lappend acc b;
   return $acc }` → `tcl opt` emits O130 and rewrites the body to
   `set $name zzz; set acc {a b}; return $acc`. `f acc` returns `zzz b` before
   and `a b` after.
2. `proc f {name} { set x 1; set $name 2; puts $x }` → O102 + O109 rewrite it to
   `set $name 2; puts 1`. `f x` prints `2` before and `1` after.
3. `proc f {name c} { set flag hello; if {$c} { set $name zzz; puts $flag } }` →
   O125 inserts `set flag hello;` immediately before `puts $flag`, *after* the
   dynamic write. `f flag 1` prints `zzz` before and `hello` after. (The paired
   O125 delete is separately suppressed by
   `manager.rs:238 drop_def_elims_resurrected_by_replacements`, so the emitted
   rewrite is insert-only — which is what makes this one visible.)

All three are quick-fix-applicable rewrites surfaced in the editor, not hints.

**What cleanup looks like:** give the four passes the same abstention
`elimination.rs:446` already has. The narrow version is a shared helper on
`FunctionUnit` (`fn dynamic_barrier_blocks_value_motion(&self) -> bool`) that
`chain_fold::protected_vars`, `code_sinking::run`,
`propagation::run_load_forwarding`, and `pattern_recognition` consult before
emitting; the precise version is to make `chain_fold::classify_write` and
`code_sinking::sinkable_assignment` return `None` for a non-literal variable
word and to fold `dynamic_names.writes` into the `escaping` set
`propagation.rs:238-241` builds (matching what `sccp_with_builtin_folds` does
with `trace.traced_variables`). This **will** change user-visible output: O102 /
O104 / O119 / O125 / O130 will stop firing in any proc that contains a computed
variable name.
**Scale:** four call sites plus one shared helper; no cross-crate change.

---

## F2: Variable-trace observability is answered by a canonical whole-module fact in SCCP/O102/taint and by two spelling-sensitive per-function scans in O109/O126, O104/O130 and W211/W220

**Confidence:** high
**Category:** half-ported

**The modern path:** `Module::traced_variables` + `Module::has_dynamic_variable_trace`
(`ir.rs:1502`, `ir.rs:1507`). Populated at lowering time by
`populate_trace_facts` / `populate_variable_trace_facts`
(`lowering/mod.rs:3427-3450`, `:3607-3633`) — registry-driven
(`Traits::ESTABLISHES_VARIABLE_TRACE` + `ArgRole::VarWrite`, no hardcoded
`trace` grammar), covering the top level, every proc, every `body_unit`, and
every TclOO method, and **`::`-canonicalised on purpose** so that a top-level
`set x 5` matches a `trace add variable ::x …` target — the comment at
`lowering/mod.rs:3618-3622` says exactly that. The field doc at `ir.rs:1488-1501`
states the intended consumers: "the propagation optimiser (`O102`
load-forwarding) **and dead-store elimination** must never treat a use of one of
these names as equivalent to its last literal assignment." It is threaded as
`sccp::TraceInputs` (`sccp.rs:295-314`) and consumed by SCCP
(`sccp.rs:395-398`, `:448`), by O102 (`propagation.rs:123-132`, `:238-248`), and
by the taint engine (`taint.rs:2134-2135`).

**The legacy/duplicate path:** dead-store elimination never received it.

- `elimination.rs:456` builds `scope_aliases` from `scan_scope_aliases`
  (`elimination.rs:1282-1310`), a per-CFG walk that inserts the trace target's
  **raw argument text** (`aliases.insert(t.clone())` at `elimination.rs:1303`).
  The O109/O126 gate at `elimination.rs:537` then compares that against the
  chain's normalised name. `manager.rs:482` builds the same set for the O109
  propagation-coupling removal and tests it at `manager.rs:523`.
- `chain_fold.rs:96-104` (`protected_vars`) uses
  `var_observability::analyse_var_observability(&fu.cfg, …).escaping_var_names()`,
  which is single-function and likewise keys on the spelled name.
- A *third* implementation exists — `scan_module_traced_globals`
  (`elimination.rs:1322-1357`), a whole-module scan restricted to `::`-qualified
  literal targets. It lives in the optimiser but its only caller is the
  **analyser** (`analyser/diagnostics.rs:454-455`, feeding W211/W220
  suppression). No optimiser pass calls it.

**Why it matters:** the spelling of the trace target decides whether the
rewrite is sound.

```tcl
proc onw {a b c} { puts trace }
trace add variable ::g write ::onw
set g 1
set g 2
puts $g
```

`tcl opt` emits **O109 "Eliminate dead store"** and deletes `set g 1`, so the
write trace fires once instead of twice. `tcl diag` on the same file emits
**W220 "Assignment to 'g' is never read"**. Change the stores to `set ::g 1` /
`set ::g 2` and both correctly go silent (O109 via the unrelated
`var.starts_with("::")` check at `elimination.rs:543`, W220 via
`scan_module_traced_globals`). Meanwhile O102 on the *same* unqualified spelling
correctly abstains — `set g 1; puts $g` under that trace is left alone, and
without the trace it forwards. Same variable, same file, one pass sound and one
not.

The list fold has the same hole: `trace add variable ::acc write ::onw; set acc
{}; lappend acc a; lappend acc b` folds to `set acc {a b}` under O130, dropping
two write-trace callbacks — and it does so even when the `trace` call is in the
*same* scope, because `analyse_var_observability` recorded `::acc` and
`chain_fold` asks about `acc`.

**What cleanup looks like:** route both the O109/O126 gate and
`chain_fold::protected_vars` through the same canonicalised fact SCCP and O102
use, i.e. widen their protected set with `cu.ir_module.traced_variables` and bail
on `has_dynamic_variable_trace`, normalising the chain key the way
`populate_variable_trace_facts` normalises the target. Because
`Module::traced_variables` is name-keyed and scope-blind, doing this verbatim
would also protect a *proc-local* `x` when some other proc traces its own local
`x`; the precision-preserving version is to promote
`scan_module_traced_globals`' scope filter into a single module-level fact
(globals-traced-anywhere ∪ this-function's-own-traces) and have all four
consumers — SCCP, O102, O109/O126, O104/O130 — read that one. This changes
user-visible output: O109/O126/O104/O130 and W211/W220 all lose firings around
traced globals.
**Scale:** one new shared fact plus four consumer call sites; contained in
`tcl-compiler`.

---

## F3: GVN's PRE and LICM define "executable block" as raw CFG reachability, so O105/O106 fire inside code that O107/O112 simultaneously propose to delete

**Confidence:** high
**Category:** duplicated-decision

**The modern path:** `SccpResult::executable_blocks` is the compiler's
reachability answer, and every sibling consumer uses it — `elimination.rs:395`
(`unreachable_blocks`, the O107 source), `propagation.rs:602` (O127 candidate
gate), `propagation.rs:1641`/`:1695`, and the analyser's
`crate::loops::build_loop_forest`, which takes the executable set as a parameter
(`loops.rs:127-131`) and is called with `fu.sccp.executable_blocks`
(`analyser/diagnostics/helpers.rs:924`). `loops.rs:19-25` states its purpose
outright: the natural-loop forest "lives in `tcl-compiler` next to the CFG/SSA so
the compiler explorer (and any future loop-aware tooling) reuse it rather than
re-deriving loops."

**The legacy/duplicate path:** `gvn.rs:1533` (`find_loop_invariants`) and
`gvn.rs:1825` (`find_partial_redundancies`) both do
`let executable = reachable_from(cfg, ssa.entry);` — `reachable_from`
(`gvn.rs:1437-1462`) is a plain terminator-successor walk with no SCCP input.
`find_loop_invariants` then re-derives the whole loop forest inline
(`gvn.rs:1545-1574`: back-edge detection, `natural_loop_blocks` at
`gvn.rs:1468-1493`, `loop_defined_variables` at `gvn.rs:1497-1515`) — a
line-for-line duplicate of `loops.rs:98-181`, differing only in that it never
learns which blocks SCCP proved dead. The doc comment at `gvn.rs:1274-1276`
records the choice ("Treats every CFG block as executable (no SCCP
unreachability filter). Callers that want SCCP pruning can pre-filter the CFG"),
and the production caller `compiler_checks.rs:358-363` passes `&fu.cfg, &fu.ssa`
unfiltered.

**Why it matters:** contradictory user-visible diagnostics on one file.

```tcl
proc f {lst} {
    set t 0
    if {0} {
        for {set i 0} {$i < 3} {incr i} {
            set n [llength $lst]
            set t [expr {$t + $n}]
        }
    }
    return $t
}
```

`tcl explore --json` reports `unreachableBlocks: 6` and, from the same run, one
`gvn` finding: **O106** "`llength $lst` is loop-invariant and re-computed on each
iteration. Consider hoisting it before the loop." `tcl opt` on the same file
emits **O112** "Eliminate dead if (all conditions are always false)" and deletes
the entire block the O106 points into. The editor therefore offers a hoist
refactor inside a region it is simultaneously offering to delete. The same
divergence lets the latch-dominance gate at `gvn.rs:1593` compute a loop body
that includes blocks reachable only through a constant-false edge, which can
also *suppress* a legitimate O106.

**What cleanup looks like:** change `find_loop_invariants` and
`find_partial_redundancies` to take the executable set (or the `FunctionUnit`)
and seed it from `fu.sccp.executable_blocks`, then delete `reachable_from`,
`natural_loop_blocks`, and the inline back-edge loop in `gvn.rs` in favour of
`crate::loops::build_loop_forest`. The `_for_cu` wrappers
(`gvn.rs:1929`, `:1947`) already hold a `FunctionUnit`, so the signature change
is local. Naturally paired with the exemplar's own PRE/LICM migration, since
both touch the same two entry points. User-visible: O105-PRE and O106 stop
firing inside SCCP-dead code.
**Scale:** two function signatures plus deletion of ~90 duplicated lines.

---

## F4: The interprocedural purity summary reaches dead-store elimination but never GVN, while `gvn.rs` carries a complete unwired second implementation of it

**Confidence:** high
**Category:** half-ported

**The modern path:** `InterproceduralAnalysis::procedures[q].pure`
(`interprocedural.rs:168`), computed by the `fixpoint_pure` least fixpoint over
local purity ∧ callee purity (`interprocedural.rs:1040-1064`, driven from
`:578-626`). It is a real, load-bearing optimiser fact: `elimination.rs:234-249`
(`pure_call_targets`) projects it into the `PurityCtx` that gates O109/O126/O108
RHS deletion (`elimination.rs:126-139`).

**The legacy/duplicate path:** `gvn.rs` ships a parallel intra-module purity
fixpoint — `PureProcs` (`gvn.rs:384`), `find_pure_procs` (`gvn.rs:437-460`),
`function_body_is_pure` / `statement_is_pure` (`gvn.rs:464-531`),
`is_pure_with_procs` (`gvn.rs:559`), and `is_worth_reporting_with_procs`
(`gvn.rs:573-592`), the last documented as "user procs proved pure by
intra-module analysis count as CSE candidates even if they don't carry the
`CSE_CANDIDATE` registry trait". A workspace-wide grep finds **zero production
callers** of any of the five — only `gvn.rs`'s own `#[cfg(test)]` block
(`gvn.rs:3306-3473`). Production GVN instead calls the registry-only
`is_worth_reporting` (`gvn.rs:668-673`, used at `gvn.rs:1043` and `:1079`), and
the modern semantic path requires `Traits::PURE | Traits::CSE_CANDIDATE` from the
registry (`gvn.rs:896-900`), which a user-defined proc can never carry.

**Why it matters:** a missed optimisation on the most valuable CSE shape in real
Tcl. For

```tcl
proc double {a} { return [expr {$a * 2}] }
proc f {x} { set p [double $x]; set q [double $x]; return [list $p $q] }
```

`tcl explore --json` reports `interprocedural[0] = {"name": "::double", "pure":
true, …}` and `gvn: []`. The compiler has *proved* the call reusable and no
redundancy pass can see it, while `elimination` in the same run happily uses that
same `pure` bit. Secondary cost: ~130 lines of untested-in-production purity
logic in `gvn.rs` that a reader will reasonably assume is live, and whose
judgement differs from `interprocedural`'s (e.g. `statement_is_pure`
`gvn.rs:501-505` treats any unqualified `def` as proc-local-and-therefore-pure,
a rule the interprocedural summary does not have).

**What cleanup looks like:** decide which purity oracle wins and delete the
other. The straightforward version is to delete `PureProcs`/`find_pure_procs`/
`is_pure_with_procs`/`is_worth_reporting_with_procs` and instead give
`GvnSemanticFacts::from_function` (`gvn.rs:763`) access to the interprocedural
summary, so an `InvocationResolution::Unresolved` head whose qualified name is in
`interproc.procedures` with `pure: true` and no world barrier becomes
`GvnSiteEligibility::Eligible` with the proc's qualified name as the canonical
key. That is the larger half: the current sidecar deliberately fails closed on
unresolved names, so the summary needs projecting into `InvocationFacts`-shaped
evidence rather than being consulted as a name set. User-visible: new O105/O106
findings on repeated pure user-proc calls.
**Scale:** a pass-level change in `gvn.rs` plus a plumbing change to carry
`InterproceduralAnalysis` into the GVN entry points; alternatively a pure
deletion if the capability is not wanted.

---

## F5: `PassContext::cross_event_vars` is written only inside `elimination::run`, so the four passes that read it always see an empty set

**Confidence:** medium
**Category:** stale-consumer

**The modern path:** the iRules cross-event / TclOO instance-state escape set is
derived from real analyses inside `elimination::run` —
`cu.connection_scope.cross_event_defs ∪ cross_event_imports` for `::when::*`
procs (`elimination.rs:296-313`) and `ir_module.methods[q].instance_vars` for
method bodies (`elimination.rs:333-340`). It is stashed into
`ctx.cross_event_vars` and consumed at `elimination.rs:551`, then restored
(`elimination.rs:307`, `:324`, `:333`, `:357`).

**The legacy/duplicate path:** `PassContext::cross_event_vars`
(`optimiser/mod.rs:254`) is documented as shared state — "names whose values must
be preserved across `when <event>` boundaries" — and is read as a safety gate by
four other passes: `code_sinking.rs:79` (O125), `pattern_recognition.rs:142`
(O119), `chain_fold.rs:74` (O104/O130), and `propagation.rs:680` (O127). But the
one production context builder, `manager::build_pass_context`
(`manager.rs:132-144`), never populates it, and `elimination` `mem::take`s and
restores it, so it is `HashSet::new()` for every reader. The `#[cfg(test)]`
comment at `optimiser/mod.rs:526` ("cross_event_vars and next_group are
intentionally preserved across functions") reads as if some caller fills it; none
does.

**Why it matters:** four passes document and test a protection that does not
exist in the shipped pipeline. The largest exposure is O125: with the
`::when::HTTP_REQUEST` handler

```tcl
when HTTP_REQUEST {
    if { [HTTP::header exists X] } { set n 1 } else { set n 2 }
    set flag $n
    if { $n > 1 } { log local0. "req $flag" }
}
when HTTP_RESPONSE { log local0. "resp $flag" }
```

`tcl opt --dialect f5-irules` emits O125 to sink `set flag $n` into the
conditional branch even though `flag` is a cross-event variable read in
`HTTP_RESPONSE`. Today the paired delete is dropped by
`manager.rs:238 drop_def_elims_resurrected_by_replacements` (the insert text
always re-mentions `$var`), so applying the surviving edit only duplicates the
assignment rather than losing it — the guard is currently latent rather than
actively wrong. That accident is exactly why it is worth fixing now: any
refinement to the FP-OPT-08 guard turns this into "variable unset on the
not-taken path" in an iRule. Confidence is medium only on the severity, not on
the fact: the field is verifiably never written outside `elimination`.

**What cleanup looks like:** populate `cross_event_vars` once in
`manager::build_pass_context` from `cu.connection_scope` (and per-function from
`ir_module.methods[..].instance_vars` where the pass walks methods), and drop
`elimination`'s local save/restore in favour of the shared value — or, if the
per-function scoping `elimination` needs cannot be hoisted, replace the shared
`PassContext` field with an explicit `fn cross_event_vars(cu, qname)` helper each
pass calls, so a reader cannot mistake an empty default for "nothing is
cross-event". No user-visible change today (the guard only ever suppresses);
after the FP-OPT-08 interaction is fixed it would suppress some O125 in iRules.
**Scale:** one context-builder change plus the removal of a misleading shared
field; a few call sites.
