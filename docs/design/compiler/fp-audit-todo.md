# False-positive audit — full code inventory & todo

A living checklist for sweeping **every** diagnostic / optimisation / shimmer
code against the real corpus (tcllib 2.0, Tcl 9.0.3 stdlib, tklib 0.9, tdom,
**SpiceGenTcl**) looking for false positives.

**Method per code:** dump every firing (`bench/fp_snippets.py`-style harness,
dialect-aware via `detect_dialect_from_source` + `dialect_scope` — see the note
below), group by site/shape, reduce the highest-volume shapes to a minimal
repro, verify against **C tclsh 9.0.3**, and either (a) fix the FP with paired
TP/FP regression tests, or (b) record "confirmed true-positive / no change"
with the reasoning.

> **Harness correctness note (learned this round):** a raw `get_diagnostics(src)`
> uses the default dialect `tcl8.6`. The corpus must be swept *dialect-aware* —
> detect `package require Tcl X.Y` / `# tcl-dialect:` and wrap in
> `dialect_scope(...)`, exactly as the LSP does — or version-gated commands
> (e.g. `oo::configurable`, a Tcl 9 command) produce phantom W002/W004 that are
> NOT real FPs. The audit harness applies this.

Legend: `[x]` inspected & resolved · `[~]` partially inspected · `[ ]` not yet
inspected · counts are dialect-aware corpus firings as of the last sweep.

---

## Resolved this audit (FP fixed, paired tests landed)

- [x] **W210** read-before-set — fixed: `dict for`/`dict map` body recovery;
  `vwait varName` exemption; cmd-sub writes inside `return`/branch terminators.
  (also covered by the RBS FP family on stage-2)
- [x] **W001** unknown subcommand — fixed: Tk geometry-manager shortcut
  (`grid .x` / `pack .x` / `place .x`).
- [x] **W306** literal-expected substitution — fixed: escaped `\[`/`\$` and
  literal `$` end-anchor in quoted regexp/regsub patterns no longer flagged
  (raw-source live-substitution scan); live `[cmd]`/`$var`/`${ns}` still fire.
- [x] **W104** string-concat list building — fixed: usage/template notation
  (`?optarg?`, `<placeholder>`, `...`) is display formatting, not a list
  element, so suppressed (corpus 165→144 sites); genuine `append x " $item"`
  list-building still fires.
- [x] **W126** non-channel value in channel arg — fixed at the **lattice**:
  `lassign` destructures list *elements* (channels in `lassign [chan pipe] ch
  wch`), not lists, so its def targets are no longer typed LIST (they stay
  UNKNOWN — the sound conservative value); corpus W126 4→0, all were this
  type-inference artifact; captured-return `set rest [lassign...]` still LIST.
- [x] **O110** Canonicalise expression (InstCombine) — fixed across four
  passes; corpus 3641 → ~700-900 (-75-80% est.).  The original baseline
  fired on every whitespace touch the rewriter performed.  Sequential
  fixes, each with paired tests:
  1. `_strip_ws` guard on the two `expression_args` /
     `expr_substitutions` paths — drops whitespace-only rewrites
     (3641→1490, −59%).
  2. Same `_strip_ws` guard on the `_branch_folding.py` path —
     `if {$x<0}` no longer flagged (bigfloat2 122→46, exif 53→10).
  3. Bitwise/shift paren-preservation in the AST renderer — keeps
     parens for mixed bitwise/shift (CERT EXP00-C; DES 91→23).
  4. Commutative-reorder suppression in `_simplify_expr_node` — the
     reassoc no longer swaps ``literal + term`` to ``term + literal``
     when no real fold would result; identities and operator flips
     still fire (bigfloat2 46→35, exif 10→4).
- [x] **W211 / W220** call-by-name suppression — fixed using the
  `ProcDef.param_traits` lattice: when a caller passes a *literal* variable
  name to a user proc whose param carries `ProcArgTrait.VAR_READ` or
  `VAR_WRITE` (a Tcl-side upvar idiom), the analyser no longer flags that
  caller-local as set-but-unused / dead — the callee operates on it
  through an upvar alias.  Substituted args (`$x`, `arr(k)`, `[..]`) are
  excluded so the suppression does not over-reach.  Tests in
  `TestCallByNameSuppression`.
- [x] **W214** unused proc parameter — fixed by detecting **dispatch
  protocols**: when ≥3 peer procs in one namespace share an identical
  leading-parameter signature (e.g. tcllib's PEG rule procs all take
  `{s e}`), those names form a contract the dispatcher relies on, so an
  individual rule body not using one is not a bug — changing the
  signature would break dispatch.  `args` (Tcl's variadic catch-all) is
  excluded from the protocol shape.  Genuine unused params *beyond* the
  protocol shape still fire.  Tests in `TestDispatchProtocolSuppression`.
- [x] **S100 / S101** loop-invariance lattice — fixed: use-site shimmer was
  unconditionally upgraded to S101 ("per-iteration cost") whenever the
  shimmering var was used inside a loop block, but a **loop-invariant**
  var (no def anywhere in the loop body, incl. phi names) shimmers
  *once* — Tcl's Obj intrep cache makes the conversion one-time.  Compute
  the per-loop def-name set and downgrade the use to S100 when the var is
  invariant in that loop.  Genuine per-iteration shimmer (loop body
  re-assigns the var to the from-type) still classified as S101.

## Confirmed true-positive this audit (sampled, no change needed)

- [x] **W304** missing `--` terminator — tclsh confirms `switch $x` / `file
  delete $f` consume a leading-dash value as an option. TP. (1453)
- [x] **W103** `open` variable arg — tclsh confirms `open "|cmd" r` pipes even
  with an explicit access mode. TP. (398)
- [x] **W212** substitution where var-name expected — `set $x` / `incr $x` /
  `lappend $x` are genuine dynamic-name foot-guns; `upvar`/`dict`/`trace`/
  `namespace which` correctly exempt. TP. (390)
- [x] **W301** uplevel multi-arg concatenation — TP (logger.tcl idioms). (291)
- [x] **W313** destructive op with variable path — TP. (95)
- [x] **W110 / O120** `==`/`!=` on strings → `eq`/`ne` — TP (near-duplicate pair;
  consolidation is a policy call, noted below). (1673 / 1515)
- [x] **W002** disabled-in-dialect — confirmed the `oo::configurable` "FP" was a
  harness artifact (dialect detection); real firings (e.g. `log` disabled) TP.

## Spot-checked, mostly TP (need a fuller sweep to be sure)

- [~] **W211** set-but-unused (589 post-callbyname) — samples were genuine
  vestigial vars (tar.tcl, ncgi.tcl).  Call-by-name covered (see Resolved);
  sweep the long tail for residual definer/lassign/upvar shapes.
- [~] **W220** dead store (578 post-callbyname) — samples genuine;
  call-by-name covered; sweep for cmd-sub / array / branch-merge shapes the
  existing recovery may still miss.
- [~] **S102** shimmer (358) — phi-merge shimmer; heavy in math (bigfloat2,
  decimal, calculus, linalg) and DES.  S100/S101 loop-invariance now fixed
  (see Resolved); audit S102 per-file clusters for genuine-vs-noise.
- [~] **W123** unresolved command (1761) — mostly real missing stubs (argparse,
  dget/dexist, custom widget cmds). Not analyser FPs, but a per-package stub
  pass would cut noise. Triage which are stdlib-ish vs project-local.

---

## NOT YET INSPECTED — optimisation hints (O-series)

These drive the optimiser view + quick-fixes; an FP here is a misleading "you
can simplify this" suggestion. None swept yet.

- [x] **O110** Canonicalise expr (InstCombine) — RESOLVED (see top):
  whitespace-only rewrites no longer emitted; corpus 3641→1490 (−59%).
- [ ] **O120** use eq/ne (1515) — pairs with W110; check the dup-with-W110 policy.
- [ ] **O100** propagate constant into arg (349)
- [ ] **O116** fold constant list command (343)
- [ ] **O105** (300)
- [ ] **O127** remove inlined assignment (496) — interacts with the dead-store /
  W220 model; verify it never suggests removing a still-live assignment.
- [ ] **O126** remove unused variable assignment (558) — same DCE family.
- [ ] **O111** brace expression text (219)
- [ ] **O101** fold constant expression (205)
- [ ] **O112** (199)
- [ ] **O109** eliminate dead code (183) — DCE; gate against O106 byte-identity.
- [ ] **O106** (149) — has a byte-identity oracle (`bench/phase1_loops.py`); use it.
- [ ] **O107** eliminate unreachable code (116) — RCH family has FP tests; re-sweep.
- [ ] **O125** (0 corpus) — verify it can still fire; synthetic test.

## NOT YET INSPECTED — style / lexical warnings

- [ ] **W111** line too long (36012) — pure length; low FP risk but confirm the
  length config + tab handling. Likely "no change".
- [ ] **W112** trailing whitespace (15609) — pure lexical; likely "no change".
- [ ] **W100** unbraced expr (219)
- [x] **W104** string-concat list building → lappend — RESOLVED (see top): usage/
  template notation suppressed; corpus 165→144.
- [ ] **W105** unbraced code-block arg (396) — INJ family has some coverage;
  re-sweep the non-eval shapes.
- [ ] **W106** dangerous unbraced switch body (0 corpus) — synthetic verify.
- [ ] **W108** non-ASCII in token (1)
- [ ] **W113** proc shadows builtin (95) — verify namespace-qualified shadowing.
- [ ] **W114** redundant nested `[expr]` (0) — synthetic verify.
- [ ] **W115** backslash-newline in comment (0) — synthetic verify.
- [ ] **W116 / W117** stub shadows builtin command/function (0) — synthetic.
- [ ] **W118** inconsistent line endings (6)
- [ ] **W120** command without package require (5)
- [ ] **W121** non-contiguous subnet mask bits (0) — synthetic.
- [ ] **W122** mistyped IPv4 (3)
- [ ] **W124** invalid IP literal (8)
- [ ] **W125** orphaned control-flow keyword (0) — synthetic.
- [x] **W126** non-channel value in channel arg — RESOLVED (see top): lassign
  element-type lattice fix; corpus 4→0.
- [ ] **W127** value not in allowed set (0 corpus, NEW from #501) — synthetic +
  corpus once a project uses a closed-set command.

## NOT YET INSPECTED — variable-shape warnings

- [ ] **W213** unset on possibly-unset var (1) — RBS-derived; re-check.
- [ ] **W215** unreachable variable name (12)
- [ ] **W216** broken brace-form array ref `${arr}(x)` (count low) — verify the
  depth-lock from OBJ family holds.
- [ ] **W240** constant-false loop condition (0) — synthetic verify.
- [ ] **W241** provably-infinite loop (0) — synthetic; intentional `while 1`
  must NOT fire (known idiom).
- [ ] **W242** loop termination unprovable (27) — sampled; sweep for
  cmd-sub-condition loops that DO terminate.

## NOT YET INSPECTED — security warnings (W3xx) + taint (Txx)

- [ ] **W302** catch without result var (660) — HINT severity; confirm the
  "cleanup idiom" (`catch {file delete}`) policy is intended noise or should
  be exempted.
- [ ] **W303** ReDoS regexp (0 corpus) — synthetic verify.
- [ ] **W307** non-literal command name (7574) — #1 W-code by volume. OBJ family
  fixed the snit/oo/factory cases; remaining is `$param method` cross-proc
  dispatch (known open — needs interprocedural object typing). Re-sweep to
  confirm no NEW shape crept in.
- [ ] **W308** subst without -nocommands (0 corpus) — synthetic.
- [ ] **W309** eval/uplevel with subst (0 corpus) — synthetic.
- [ ] **T100** tainted → code-exec sink (0 corpus) — synthetic.
- [ ] **T101** tainted → output sink (count low)
- [ ] **T102** tainted in option position (2) — INJ family has position-aware
  fix; re-verify.

## NOT YET INSPECTED — errors (Exxx) + hints

- [ ] **E001** missing subcommand / **E002** too few args / **E003** too many
  args — arity. Sweep for custom-arity commands (ensembles, varargs) that may
  miscount. (E002/E003 fire in corpus.)
- [ ] **E004** malformed control flow (0) — synthetic.
- [ ] **E200** shimmer parse error (0) — synthetic.
- [ ] **H300** possible paste error (0 corpus) — synthetic.

## NOT YET INSPECTED — iRules (IRULE*) + Tk (TK*)

The corpus is mostly non-iRules/non-Tk, so these barely fire here. Need a
dedicated iRules corpus + the Tk stdlib for a real sweep.
- [ ] **IRULE1001** (1) / **IRULE1005** (2) — only ones firing; rest 0 here.
- [ ] **IRULE1002–5007** — need an iRules corpus.
- [ ] **TK1001/1002/1003** geometry/parent/option (0 corpus) — the Tk geometry
  W001 fix added coverage; sweep a real Tk app for TK100x FPs.

---

## Cross-cutting follow-ups (known, not yet done)

- [ ] **W210 `$dir` in pkgIndex.tcl** (~196 firings, the single biggest W210
  cluster) — Tcl's package machinery sets `$dir` before sourcing; needs a
  uri-gated implicit-var at the diagnostic layer (`get_diagnostics(uri=...)`).
  LSP-level, deferred.
- [ ] **W110 / O120 near-duplicate** — 1020+ ranges are byte-identical between
  the two. Policy call: which subsystem owns the user-facing squiggle.
- [ ] **W123 per-package stubs** — argparse / dict-extension (dget/dexist) /
  custom widget commands. A stub bundle would cut ~half the W123 noise.

## Process

- Sweep highest-volume un-inspected first (O110, then W104/W126 as likely-FP,
  then the O-series DCE family, then the long tail).
- Every behaviour change: paired TP/FP tests (mirror the FP catalog convention),
  tclsh-verified, ci-fast + the relevant suite, then test-slow stamp.
- Record confirmed-TP outcomes here too (negative results are results).
