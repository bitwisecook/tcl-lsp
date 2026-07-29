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

# Status note (2026-05-30): all resolved items below now have formal
# FP.md catalog entries with reproducers, evidence, and paired TP+FP
# tests.  Mapping:
#
# OPT family (4 entries, 10 tests) — O106/O109/O110/O116/O126
# TNT family (2 entries, 6 tests)  — T100/T101
# STY family (8 entries, 23 tests) — W001/W104/W120/W122/W124/W126/W214/W302/W306
# NAB extensions (7 entries, 10 tests, NAB-04..10) — W110/W304/W103/W313/W212/W301/W002
# OBJ extensions (3 entries, 6 tests, OBJ-09..11) — W307 multi-dispatch / switch-callback / factory
# SH extensions (3 entries, 7 tests, SH-04..06) — hex/binary literals, destructure foreach, per-loop body_types
# OBJ extensions (2 entries, 4 tests, OBJ-07..08) — W307 cmd-sub namespaced ensemble, W307/W101 dedup
#
# 177 paired FP tests passing (+ 2 expected xfails for genuine
# open findings: namespace upvar, ARRAY_ELEM dead-store).  Catalog
# is now complete for every resolved item in this checklist.
#
# Update (2026-07-14): the two remaining open findings above are CLOSED in
# the Rust analyser — see the "namespace upvar" and "ARRAY_ELEM dead-store"
# entries at the end of "Resolved this audit".  No expected xfails remain;
# the Rust FP suite carries no `#[ignore]` for either finding.

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
- [x] **O116** `Fold constant list command` — FIXED A CORRECTNESS BUG:
  empty `[list]` folded to the Python empty string, so applying the
  quick-fix on `set r [list]` produced `set r ;` (a syntax-valid READ
  of `r`, not the intended write).  Use `{}` (canonical empty-list
  literal) so the apply preserves the assignment.  All 346 corpus
  firings now apply cleanly.
- [x] **O109 / O126** Eliminate dead store / Remove unused variable
  assignment — extended the **call-by-name** suppression (already on
  W211/W220) to the optimiser's DCE pass.  Both emitters previously
  ignored ProcDef.param_traits and would delete a `set x …; reader x`
  store whose value is consumed indirectly via upvar in the callee.
  Extracted the helper to `compiler/proc_arg_traits.py` so both layers
  share one source of truth.  Known precision gap (documented): an
  earlier-version dead write to a var later passed by name is
  conservatively suppressed (sound, loses one TP class; would need
  version-precise reaching-uses through the call site to tighten).
- [x] **O106** `Hoist loop-invariant computation` — the LICM purity
  check only inspected the OUTER command name and treated args as
  opaque strings.  Outer-pure-but-inner-impure expressions like
  `[format %04d [incr testnum]]` (corpus: `clay/build/test.tcl L686`)
  and `[read $fh 512]` were flagged as hoistable — applying the hoist
  would change runtime semantics (incr would only fire once, read
  would lose its per-call consumption).  Now `_is_pure_command`
  recurses into argument command substitutions; any inner impure
  command marks the whole expression impure.  `_parse_cmd_token` now
  re-wraps CMD-sub arg pieces in `[...]` so the recursion can see
  them.  Paired TP test (outer pure + inner pure still fires).
- [x] **W211 / W220 / O109 / O126** call-by-name inside cmd-subst —
  the previous call-by-name fix only scanned top-level `IRCall` /
  `IRBarrier` statements.  But the dominant tcllib shape is a literal-
  name arg inside a `[..]` substitution: `set len [asnPeekTag data
  tag type dummy]` (asn module), where `tag` / `type` are caller-
  locals consumed indirectly via `upvar` in `asnPeekTag`.  Extended
  `collect_call_by_name_reads` to also scan command substitutions
  inside `IRAssignValue.value` (raw text → TclLexer for CMD tokens,
  recursing) and inside `IRAssignExpr` / `IRExprEval` / `IRReturn`
  expr trees (walking `ExprCommand` nodes).  Sample asn.tcl: W211
  2→0.  Both analyser- and optimiser-side benefit automatically.
- [x] **W302** catch without result variable — `catch {<cmd>}` without
  a result var is the documented Tcl idiom for "do this if possible,
  ignore if not".  The corpus shows 239 firings (top: ftp.tcl 35,
  comm.tcl 19, http.tcl 16); spot-check confirmed every single one is
  this idiom (``catch {after cancel $h}``, ``catch {file delete}``,
  ``catch {close $fh}`` etc.).  Add a `_FIRE_AND_FORGET_COMMANDS` set
  covering the canonical "error-on-missing-target" builtins (`after`,
  `close`, `chan`, `unset`, `array`, `dict`, `interp`, `namespace`,
  `rename`, `file`); a single-statement catch body whose head matches
  is exempt.  Multi-cmd bodies and user calls still fire.
- [x] **W214** empty-body stub procs — `proc foo {a b} {}` with an
  empty body is the canonical Tcl signature-stub pattern.  tcllib's
  `grammar_fa/faop.tcl` declares 14 such empty-body procs as the FA
  algebra API (overlay files plug in real bodies later).  Every
  parameter is necessarily "unused" because there is no body to use
  it.  Detect via the IR: zero-statement body skips W214 entirely.
  Sample faop.tcl: 19 W214 firings, all empty-body stubs.
- [x] **W214** snit-style quoted-keyword marker params — snit's
  command DSL uses `{"as" ""}` as a positional keyword marker (the
  param name is the literal `"as"`).  The body cannot USE a quoted-
  name param as a variable; flagging is noise.  Detect via param name
  starting + ending with `"`.
- [x] **W302** subcommand-aware fire-and-forget — the initial fix was
  too broad for ensemble heads; ``catch {chan close}`` is the
  fire-and-forget idiom but ``catch {chan configure}`` should fire.
  Split into `_FIRE_AND_FORGET_BARE` (`close`/`unset`/`rename`) +
  `_FIRE_AND_FORGET_SUBCOMMANDS` (after-cancel, chan-close, array-
  unset, dict-unset, interp-delete, namespace-delete/forget,
  file-delete).  Constructive subcommands (`array set`, `dict set`,
  `file copy`, `chan configure`, `namespace eval`, `interp create`,
  `after <ms>`) still fire W302.
- [x] **T101** puts channel-vs-output filter — `puts ?-nonewline?
  ?channelId? string` was firing T101 on the channel-id arg (the
  destination handle, not the content).  tcllib imap4.tcl ``puts
  -nonewline $chan "$t\r\n"`` cleared 3 firings.  Only the trailing
  positional arg can carry injectable content; filter T101 emission
  to that index on `puts`.
- [x] **T100** direct-operand expr filter — ``expr {[string length
  $data] / 8}``: $data is consumed by the inner ``string length``
  as an argument, never re-parsed as expr text.  T100 was firing on
  every tainted ``uses`` entry regardless of position in the parsed
  expr AST.  New ``_direct_expr_operand_names`` walks the ExprNode
  tree and collects ExprVar names OUTSIDE any ExprCommand subtree
  (the cmd-sub boundary).  T100 still fires on direct operands
  (``expr {$data + 1}``, ``expr {abs($data)}``) — the genuine
  injection vector for unbraced expr.  Sample tcllib firings cleared:
  blowfish.tcl:L525, http.tcl:L4338, mime.tcl:L1962-on-$X.
- [x] **W307** proc-parameter / multi-dispatch / switch-callback /
  namespaced-ensemble object dispatcher — Four-tier suppression:
  (a) ``proc walk {tree} {$tree visit $n}`` — the param itself
      documents the API contract.  Single dispatch on a param is
      enough.
  (b) ``proc analyze {G} { set TGraph [createTGraph $G]; $TGraph
      node first; $TGraph dispose }`` — multiple dispatches on the
      same local var (≥2) demonstrate firm intent; the user
      designed it as an object handle.
  (c) ``$state(-command) $token`` (switch-style) or
      ``$state(openCmd) $arg`` / ``$state(doneCallback)`` (suffix-
      style) — array-element callback.  The dash-prefixed key OR a
      key whose final word is `cmd`/`command`/`callback`/`handler`/
      `hook`/`proc` (case-insensitive) marks the slot as an
      explicitly-registered command.
  (d) ``${log}::debug "msg"`` (VAR-sub form) and ``[[namespace
      parent]::outputChannel]`` (CMD-sub form) — namespaced ensemble
      dispatch where the namespace prefix comes from a variable or
      command substitution, ``::tail`` literal forms the qualified
      command path.  tcllib logger / dns / spf / irc / multiplexer
      use the VAR form; tcl9 tcltest.tcl uses the CMD form.
  Track per-proc dispatch counts in the pre-pass over
  `_var_command_sites`; suppress on param / multi-dispatch local /
  switch-key array elem / namespaced ensemble / **param-array
  base** (``\$Verify(key)`` where ``Verify`` is itself a proc
  parameter — the param documents the callback-table contract).
  W307 also **dedups with W101** (eval-injection): ``eval \$cmd``
  fires both at the same start offset; W101's more-specific
  double-substitution warning is kept and W307 is dropped.  **Top-level scope**
  also honours the multi-dispatch rule (tcllib's
  ``examples/irc/mainloop.tcl`` script registers ``$cn`` and
  dispatches 9 times — clear intent regardless of being outside any
  proc).  **`::proc`** is recognised as proc declaration (clay module
  qualifies the call to bypass overrides; semantically identical to
  bare ``proc``).  **Interprocedural object-factory tracking** —
  fixpoint inference identifies user procs whose return value is
  itself an object (direct ``return [namespaced::cmd]`` OR ``return
  $X`` where X was assigned from a factory OR transitively from
  another OBJECT-RETURNING user proc).  Callers ``set Y [factory_proc
  ...]`` then dispatch on ``$Y`` are suppressed.  graphops.tcl: 88 →
  0 (every ``createTGraph``/``createResidualGraph`` use cleared).
- [x] **W122 / W124** OID-like dotted chains — `1.3.6.1.4.1.4203.1.11.3`
  (LDAP PEN OID) was being flagged as IPv4 with octet 4203 exceeding
  255.  The regex matched embedded 4-component slices of longer
  dotted chains.  Add a skip: when the matched quad is preceded by
  `.<digit>` OR followed by `.<digit>`, it's part of a longer chain
  (LDAP / SNMP OID), not an IPv4 literal.  Applied to both regex
  (W122) and SSA (W124) paths.
- [x] **W120** self-call from package providers — a file declaring
  `package provide X` is X's own implementation; `X::foo` calls
  inside it don't need `package require X`.  Union the file's
  `package_provides` into the imported-set check.  Sample: msgcat
  self-calls 2→0, fileutil/mime similar.
  **Taint-aware**: never suppress when the dispatched var is tainted
  in its proc (``set cmd [gets stdin]; $cmd op1; $cmd op2`` is a real
  injection vector regardless of dispatch count) — wired to the
  existing taint lattice that drives T100/T101.  Sound — evidence is
  the dispatches themselves, in the same proc, and taint disqualifies
  the heuristic.  Corpus W307 (800 files): **2912 → 444 (-85%)**.
- [x] **FP-RBS-05** `namespace upvar` alias handling (incl. array-element
  sources) — CLOSED (2026-07-14).  tclsh 8.6 + `tclNamesp.c
  NamespaceUpvarCmd` confirm the semantics the analyser must honour: the
  otherVar resolves via `TclObjLookupVarEx(..., createPart1=1,
  createPart2=1)` in the target namespace, so **`arr(k)` is a valid source**
  (created on demand; reads/writes through the alias hit `ns::arr(k)`;
  linking alone does not create it — `info exists alias` stays 0 until the
  target exists); the local side must be a scalar (`TclPtrMakeUpvar` rejects
  `myarr(k)`).  The Rust analyser routes the alias-local recognition through
  the shared `var_scoping` pair grammar (`scan_scope_aliases`, the
  W210 scope-alias suppression + dynamic-target override, and
  `var_observability::stmt_gen`), which covers scalar AND element sources —
  the Python port's missing-def W210 FP does not reproduce, and the feared
  shimmer cascade does not manifest (aliases are structurally recognised,
  not def-inferred).  One residual defect did survive: the lowered
  `namespace upvar` Call carries no `Call::defs` (unlike
  `global`/`variable`/`upvar`), and `sccp::existence_constant_branches`
  keyed defined-ness off defs alone, folding `[info exists alias]` to
  constant-false — I230 + an O101 DCE miscompile-hint on the exact
  `::safe::CheckInterp` guard shape (safe.tcl:109) the FP.md entry cites.
  Fixed at the fold: scope-alias locals never fold (their existence tracks
  the linked variable).  Paired tests:
  `fp_rbs_05_namespace_upvar_array_element_silent` (+ write-only FP guard,
  `$other` TP control, I230 guard pair) and
  `info_exists_does_not_fold_scope_alias_locals` /
  `info_exists_fold_survives_unrelated_scope_alias` (fold-alive control).
- [x] **FP-DS-06** ARRAY_ELEM same-element dead-store — CLOSED (2026-07-14).
  The Python-era xfail (`test_FP_DS_06_same_element_overwrite_still_fires`)
  tracked the missing must-alias kill; the Rust analyser has it:
  `place_bridge::must_alias_killed_in_block` overrides the element-observed
  suppression for a straight-line overwrite of the same literal-key element
  with no intervening read, so `set a(k) 1; set a(k) 2; return $a(k)` fires
  W220 on the first write (O109 has the elimination-side pair).  tclsh 8.6
  evidence: the proc returns 2 and a write trace fires for BOTH writes —
  without a trace/alias the first write is unobservable, a genuine dead
  store; distinct keys (`a(k)`/`a(j)`) remain independent live slots.
  Cross-block and hidden-read same-key overwrites deliberately degrade to
  conservative silence (sound; the kill is per-block by design).  New
  adjacent FP guards: a cmd-sub read between the writes cancels the kill
  (`fp_ds_06_cmdsub_read_between_same_element_writes_cancels_kill`) and a
  traced array's overwrite stays silent
  (`fp_ds_06_traced_array_same_element_overwrite_silent`), joining the
  existing `fp_ds_06_*` pair, `state.rs::w220_must_alias_kill_same_array_element`
  and `elimination.rs::o109_array_element_*`.
- [x] **FP-IPCP-01** I230 on a genuinely alternating recursive parameter —
  CLOSED (issue #969). `compilation_unit.rs`'s interprocedural
  `param_constants` SCCP seed (`collect_call_site_constants` /
  `params_constants_from_call_sites`) trusted a proc parameter as a
  compile-time literal whenever every call site *the scan could resolve*
  passed the same value. Two ways a real, varying call site could go
  unresolved (and so silently vanish from the "every caller agrees"
  evidence) both reproduced the exact reported shape (`if {$count & 1}`
  folding to a fixed boolean on a recursive, alternating counter):
  1. **Namespace-blind resolution** — a proc declared inside `namespace
     eval` recursed into itself by its bare name; the old resolver only
     tried global-qualified spellings of the command word, so it never
     matched the proc's namespaced qualified name and the recursive
     self-call (necessarily non-literal) was dropped, leaving only the one
     external caller's literal. Fixed by routing resolution through
     `crate::interprocedural::resolve_internal_call` (the same
     namespace-relative, existence-checked resolver the analyser and
     optimiser already use), evaluated in the *calling* function's own
     namespace.
  2. **Body-argument blindness** — a call embedded inside a `catch { … }`
     / literal `uplevel { … }` body is a real caller, but the body is an
     `ArgRole::Body` argument of a *builtin* (never a user proc), so a
     flat, one-level `Statement::Call`/`Barrier` walk resolved the builtin,
     found no matching proc, and never noticed the nested call. Fixed by
     recursing into `ArgRole::Body` arguments (registry-driven via
     `CommandRegistry::arg_indices_for_role`, re-segmented with
     `crate::segmenter`, capped at `MAX_CALL_SITE_BODY_DEPTH`), the same
     primitives `interprocedural::scan_source_for_calls` already uses for
     its own call-graph walk.
  Two further instances of the exact same "the scan's completeness is
  unproven" failure were found (and closed) by post-fix audit, before either
  was reported by a user:
  3. **`TclOO` method bodies** — methods are built in a *separate* pass
     (`FunctionUnit::build_method_units`, always seeding its own
     `param_constants: None`) that runs *after* `collect_call_site_constants`
     and was never in its scan set at all — a call from inside a method body
     to an ordinary user proc was invisible regardless of namespace. Fixed by
     `build_extra_call_site_scan_contexts`: bare CFGs (no further analysis)
     for every method and synthetic body unit (`apply` lambda, `namespace
     eval` body), fed into the same scan as additional callers. Required
     hoisting `cfg_context`'s construction earlier in `build_for_inner` (it
     already existed for the methods/body-units *analysis* pass; the scan
     now shares it rather than rebuilding it).
  4. **`namespace import` aliasing** — `resolve_internal_call` only tries the
     calling function's own namespace and the global one; a call reached
     only via an imported bare name (`namespace import ::lib::helper`, or a
     `::lib::*` wildcard) resolved to nothing and vanished from the
     evidence. Fixed by `resolve_via_namespace_import`, a fallback consulted
     when direct resolution fails, reusing `ir::Module::namespace_imports`
     (already recorded at lowering time for a documented future use this is
     the first consumer of).
  Also added, since every instance above is the same class of gap: a
  `!command_mutations.trusts_proc_binding(qname)` gate (reusing the
  optimiser's own O103 rename/`interp alias` trust lattice —
  `command_binding::scan_module_command_mutations` — instead of a second,
  divergent one) and a `package provide`-in-file gate (a package file's
  procs may be public API another file calls with a different literal,
  invisible to this single-file compilation unit). A speculative
  module-wide "any dynamic `$cmd` dispatch anywhere disqualifies every
  seed" wildcard was tried and **reverted** — it broke a genuine,
  already-covered TP (`fp_var_as_cmd_param_flow_non_command_fires`, a
  dispatch-table proc whose own body legitimately uses `$cmd`) for a
  residual soundness gap that is orthogonal to what #969 actually
  reported. Every one of those gaps is now closed precisely — not by a
  wildcard — in **FP-IPCP-02**.
  **Residual gaps at the time** (listed so they aren't mistaken for
  silently-missed; all since closed): cross-file soundness for a plain
  (non-`package provide`) file `source`d by another that calls its procs
  differently (issue #977); dynamic dispatch (`$cmd args`, issue #976);
  `CommandPrefix`-role indirection (`trace add variable … command cb`,
  `-command` callback options, issue #978). Still open: `namespace ensemble
  configure -map` redirection (issue #979).
  Traced end-to-end: the same `SccpResult` feeds I230, the optimiser's O101
  fold and O107 dead-code suggestions (all suggestion-only text rewrites,
  never applied to the compiled CFG/IR — confirmed codegen/`tcl-vm`/WASM
  never consume SCCP at all and are structurally immune), so fixing the seed
  corrects every consumer at once. Tests: `compilation_unit.rs`'s
  `call_site_param_constants` module (15 cases spanning TP/FP/TN/FN across
  namespaces, recursion, `TclOO` methods, `namespace import` (exact +
  wildcard), rename/`interp alias`, dynamic dispatch, `package provide`, and
  nested `catch`/`uplevel` bodies), native e2e
  (`diagnostics::namespaced_recursive_proc_parity_check_does_not_fire_i230`
  and siblings), and a VS Code integration suite (`issue969.test.ts`).
  **PR #970 review follow-up (Codex, all tclsh8.6-verified live before
  fixing):** three more confirmed instances of the identical class of bug,
  each closing a way "resolve the callee, then trust its own qname's
  namespace for the recursion/scan context" was itself wrong, not just
  incomplete:
  - **`TclOO` method bodies resolve bare commands against global, never the
    class's namespace** — `method go {} { helper }` calls `::helper` even
    when `::Widget::helper` exists and is exactly what naively deriving the
    caller's namespace from the method's own qualified name would try
    first. `build_extra_call_site_scan_contexts` was misattributing the
    method's call to the wrong (never-actually-invoked) proc while
    simultaneously losing it as evidence for the real target. Fixed by
    forcing the caller-context string (not the CFG's own identity) to the
    same global-resolving pseudo-qname `"::top"` already uses.
  - **A `namespace eval NS { … }` body unit's synthetic qname never encoded
    `NS`** — `register_body_unit`'s `::{label}#{n}` scheme reduced every
    namespace-eval body's *resolution* namespace to global regardless of
    its real target (coincidentally correct for `apply`'s common
    global-default form, silently wrong for namespace eval). Fixed by
    threading the real target namespace (already computed in
    `lower_namespace_eval` as `child_ns`, used correctly for nested `proc`
    *registration* but never for the body unit's own qname) through via
    `join_namespace(&child_ns, "namespace-eval")`, so the qname's enclosing
    namespace — the same "everything before the last `::`" convention every
    proc/method qname already relies on — is the block's actual target.
  - **`package provide` detection was a raw-text substring scan** — over-
    triggered on the phrase merely appearing in a comment/string (needlessly
    disabling the seed) and under-triggered on unusual-but-valid spellings
    (`package\tprovide`, `::package provide` — silently *reopening* the
    exact cross-file gap the guard exists to close). Fixed by
    `has_package_provide_statement`, a recursive walk of the lowered IR
    (top level, every proc/method/body-unit, and every nested control-flow
    body) checking for a resolved `Call`/`Barrier` statement whose command
    is `package` with a literal `provide` first argument.
  **Newly confirmed (not merely theoretical) by the same review, deliberately
  left open:** `uplevel #0 { … }` also resolves against global
  (tclsh8.6-confirmed), and reproduces the identical misattribution +
  phantom-fold pair as the `TclOO`/namespace-eval cases — but the scan never
  reaches that body as an `UpFrame` statement at all. `Statement::UpFrame`
  keeps its body as a nested `Script` that CFG construction does *not*
  flatten into blocks (re-confirmed during the #976 work, correcting this
  entry's original "is inlined into CFG blocks" reading), so the walk over
  `block.statements` skips it and the call is seen only through the
  enclosing `proc` statement's own `ArgRole::Body` argument, re-segmented in
  the frame that `proc` statement sits in — the declaring namespace, not
  global. Fixing it properly needs the registry to say *which* argument of a
  frame-shifting command is its level (`uplevel`'s level detection is a
  private spec helper today, and the body-text recursion sees only raw
  words), so the scan can switch its resolution context to global for an
  absolute `#0` — a registry extension, not the one-line namespace-context
  overrides above, so left as a pinned, `#[ignore]`d regression
  (`uplevel_zero_body_resolves_against_global_not_enclosing_namespace`) for
  follow-up rather than a rushed fix. `uplevel N` for any relative
  (non-`#0`) level remains a separate, permanent approximation: the target
  frame's namespace depends on the live call stack, which is undecidable by
  a single-file static analysis. Tests: 4 new cases in the same
  `call_site_param_constants` module (19 passing + 1 pinned `#[ignore]`),
  plus the `namespace_eval_body_unit_does_not_change_bytecode` regression
  in `regex_source.rs` updated for the corrected qname format.
- [x] **FP-IPCP-02** I230 on a plain library file `source`d by a caller with
  a differing literal — CLOSED (issue #977). FP-IPCP-01's `package
  provide`-in-file guard covered a package file, but not the more common
  shape the issue reported: `lib.tcl` with **no** `package provide`, whose
  two visible callers both pass `"prod"`, `source`d by `main.tcl` which calls
  `helper dev`. `CompilationUnit::build_for` is single-source-text by
  construction, so that caller is invisible and `mode` folded. Closed on
  three fronts, all in the new `tcl-compiler/src/unit_scope.rs` (which also
  lifts `ArgConsts` / `collect_call_site_constants` /
  `params_constants_from_call_sites` / `build_extra_call_site_scan_contexts`
  out of the 2 900-line `compilation_unit.rs`):
  1. **Cross-file evidence.** `unit_scope::scan_source_call_sites` runs the
     *identical* lowering → CFG → `record_call_site_evidence` walk over
     another file's source, resolving each call against the whole project's
     proc names, and `CallSiteEvidence::merge_from` folds the result into the
     unit's own evidence before seeding. Merging is monotone (more values,
     more unknowns, more observed argument counts), so extra evidence can
     only ever *retract* a fold — never manufacture one. Plumbed through
     `UnitBuildOptions::external_call_sites`; salsa-side by
     `tcl_lsp_db::file_call_site_evidence` → `project_call_site_evidence` →
     `file_external_call_sites` (sliced to the file's own declarations, so a
     call-site edit in one file re-sets only the file that *defines* the
     callee) onto `SourceFile::external_call_sites`, which the server
     compare-then-sets in `sync_cross_file_evidence`. The `tcl` CLI does the
     same across a multi-file `diag` / `validate` invocation.
  2. **Registry-declared unit boundaries.** `has_package_provide_statement`
     knew one command pair by name; `unit_scope::scan_unit_linkage` asks the
     registry instead (`CommandRegistry::unit_linkage`, resolving the
     subcommand word so `package provide` is a boundary and `package names`
     is not). Three new `Traits` bits carry the fact as spec data:
     `PROVIDES_PACKAGE` (`package provide` / `ifneeded`), `EXPORTS_COMMAND`
     (`namespace export`, `namespace ensemble`), and `LOADS_EXTERNAL_UNIT`
     (`source`, `load`, `package require`, `auto_load`, `auto_import`).
     The two kinds are gated differently, because a host's enumeration can
     only bound one of them: `PROVIDES_PACKAGE` / `EXPORTS_COMMAND` publish
     this file's commands to consumers that need not be in the project at
     all (another checkout can `package require` it), so they decline the
     seed **unconditionally**; `LOADS_EXTERNAL_UNIT` names a caller the
     project normally does contain, so it declines only without a
     cross-file view. `namespace import` is
     deliberately *not* a boundary — it is as often an intra-file convenience
     over a namespace the same file defines, and the evidence scan already
     models the import as a real caller path.
  3. **Indirect callers.** `record_indirect_callers` records an *opaque
     caller* — "a call site exists whose arguments I do not know" — for a
     deferred command prefix (`ArgRole::CommandPrefix`: `after 0 helper`,
     `trace add variable … helper`, `-command helper`) and for a
     `CommandTableEffect::RenamesCommands` / `CreatesAliases` word naming a
     known command. That closes FP-IPCP-01's documented `CommandPrefix`
     limitation for both scans at once, and gives cross-file `rename` the
     coverage `command_binding`'s single-file trust lattice cannot have.
  Also fixed here: only a **trailing** `args` is Tcl's variadic catch-all
  (`TclCreateProc`, `generic/tclProc.c`), so `proc f {args x}`'s `x` is no
  longer skipped. The three traits above are declared through the
  `declare_traits!` macro PR #1034 introduced (issue #1031), so they carry no
  hand-written bit number and `Traits::iter_names` renders them in the
  explorer with no second name table to maintain. **Accepted residual:** the workspace *is* the
  trust boundary — a caller outside it (a `source` target not in the project,
  another project `package require`ing this one) is still unenumerable, which
  is why `PROVIDES_PACKAGE` / `EXPORTS_COMMAND` decline the seed regardless
  of what evidence a host supplies. Visible in the compiler explorer's new **Unit Scope**
  view (`unitScope`), which shows the boundaries crossed, whether a
  cross-file view was supplied, and the per-position seed verdict. Tests: the
  `call_site_param_constants::cross_file` module (8 TP/FP/TN/FN cases) plus
  `exported_namespace_declines_the_seed_even_with_a_workspace_view`,
  `unit_scope`'s own 8-case suite over the evidence primitives and the gate,
  `tcl-lsp-db`'s `cross_file_call_sites` module (4, including a backdating
  guard), four native e2e cases in `diagnostics.rs`,
  `cli::diag_shares_call_sites_across_inputs`, two explorer serialisation
  tests, and the VS Code `issue977.test.ts` suite.

- [x] **FP-IPCP-02** I230 on a parameter reached through dynamic dispatch —
  CLOSED (issue #976). FP-IPCP-01 closed every way a *literal* call site
  could go unresolved, but a call dispatched through a variable (`set cmd
  helper; $cmd dev`) was skipped outright: it counted neither for nor
  against any callee, so a proc whose only *visible* callers agreed on a
  literal was still seeded with it even when the dispatch demonstrably
  reached that same proc with a different one. The scan's completeness
  claim was therefore unproven for every module containing an indirection.
  Fixed by extracting the whole scan into `tcl-compiler/src/call_site_scan.rs`
  and resolving indirection **by value** rather than skipping it:
  1. **Dispatch value sets.** The literal strings a dispatch word can hold
     are enumerated from the enclosing scope's own literal assignments
     (`AssignConst`, plus the plain-bareword `AssignValue` shape lowering
     leaves alone) unioned, when the word names one of the body's
     parameters, with the literals its callers pass at that position. A
     word that resolves to a known set of names is recorded as an ordinary
     call site for **each** of them, so a dispatch that agrees with every
     other caller keeps folding — the per-target precision the reverted
     PR #970 wildcard lacked. Which words *write* a variable is registry
     data (`ArgRole::VarWrite`, plus `Traits::CREATES_SCOPE_ALIAS` for the
     vararg `global`/`variable`/`upvar` forms the role query does not
     expand); no command name appears in the scan.
  2. **A monotone fixpoint, not the SCCP result.** A parameter's value set
     comes from the call-site evidence the scan itself produces, so the
     rounds start from "no callers seen" and re-derive the whole evidence
     set until it stops growing (`MAX_CALL_SITE_SCAN_ROUNDS`). Each round
     is monotone in its input (values only union, unknown flags only set),
     so the chain increases to a fixpoint at which the value sets and the
     evidence agree — the circularity the issue warned about is resolved
     without ever consulting the SCCP lattice this seed feeds. Rounds run
     only when a value set was actually consulted, so an indirection-free
     module still costs exactly one walk.
  3. **Unreadable channels withdraw every seed.** When a dispatch word's
     value set cannot be enumerated (`set cmd [gets stdin]`, a
     namespace-qualified `$::cmd`, a parameter of an untracked `TclOO`
     method / `apply` lambda body), or a script reaches a command as a
     *value* rather than as text (`eval $script`, `catch $body`, `apply
     $fn`), the evidence is flagged `opaque_callee` and
     `params_constants_from_call_sites` returns `None` for the whole
     module. A body written literally — including the overwhelmingly
     common one that merely *mentions* a variable, `catch {puts $x}` — is
     still walked in place; the discriminator is
     `value_shapes::is_pure_var_ref` / `parse_command_substitution` ("the
     whole word is one substitution"), not "the word contains a `$`".
  4. **`CommandPrefix` callbacks and user-proc invokers** (issue #978, the
     residual gap FP-IPCP-01 documented) close on the same machinery: a
     `-command cb` callback's target is invoked with arguments the *runtime*
     appends, so every parameter of the named proc is poisoned (and only
     that proc's — the poison is per-target); a `[list cb $x]` prefix is
     destructured through `Traits::BUILDS_COMMAND_PREFIX`; and a
     `Traits::INVOKES_USER_PROC` head (the iRules `call PROC …` form)
     records the tail as the callee's real arguments.
  Also centralised while here: `value_shapes::whole_word_scalar_var_name`,
  the one place that answers "is this whole word just a variable, and which
  one" for value-set analyses, replacing what would have been a fifth
  hand-rolled `$name`/`${name}` parser. Tests: `unit_scope.rs`'s own
  unit suite (11 cases over the evidence map, the value-set facts, and the
  prefix-head/whole-substitution shape helpers), 14 further TP/FP/TN/FN
  cases in `compilation_unit.rs`'s `call_site_param_constants` module
  (differing literal, agreeing literal, branch-joined value set, unrelated
  target, unenumerable word, parameter dispatch tables in both directions,
  namespace variables, `namespace import` aliases, untracked method
  parameters, literal vs dynamic `apply`, callback prefixes built and
  bare), 5 native e2e cases
  (`diagnostics::dynamic_dispatch_with_a_differing_literal_does_not_fire_i230`
  and siblings), and a VS Code integration suite (`issue976.test.ts`).
  **Residual gaps, unchanged by this entry:** the cross-file `source` one
  above; `namespace ensemble configure -map` redirection; a computed head
  that resolves to a variable-*writing builtin* (`set cmd set; $cmd x 5`),
  which would need a builtin's own name to be among the literals a local
  holds; `uplevel`'s cross-frame writes, handled conservatively by making
  every value set unenumerable in a module that shifts frames rather than
  modelled per-variable; and the global `unknown` handler (a module that
  both defines `proc unknown {cmd args}` and seeds another procedure would
  need every unresolved command word counted as a call to it — the registry
  fact that would mark the handler needs the `Traits` bitfield widened past
  its current full 64 bits, so it is recorded rather than rushed).
  `namespace unknown` / `package unknown` handlers are already covered:
  the registry declares their handler argument `ArgRole::CommandPrefix`.

- [x] **FP-IPCP-03** A procedure invoked only through a `CommandPrefix`
  callback is invisible to interprocedural analysis — CLOSED (issue #978).
  Filed as a sibling of #976 on the reading that
  `interprocedural::scan_source_for_calls` "also only recurses into
  `ArgRole::Body`, never `ArgRole::CommandPrefix`". Measured rather than
  assumed, that turned out to be only partly true, and the two halves needed
  different work:
  1. **The call-site *seed* half had the whole gap** and is closed by
     FP-IPCP-02's machinery — see its point 4.
  2. **The call-*graph* half already recorded a bare `-command cb` edge**
     (`scan_call_facts` has consulted `CommandRegistry::command_prefixes`
     since PR #915), so a callback-only proc was already not dead code. What
     it missed was the *built* prefix: `-command [list cb $x]` read its head
     as `[list`, failed the bareword guard, and recorded nothing. Confirmed
     with a probe before fixing (`bare → ["::cb"]`, `built → []`).
  The fix answers the issue's own "share one primitive or fix each
  independently" question in favour of sharing: `interprocedural::
  command_prefix_head` is now the one place that reads a callback prefix's
  head — bareword, braced list, or built by a
  `Traits::BUILDS_COMMAND_PREFIX` command — consumed by both the call-graph
  builder and `call_site_scan`. Fixing them independently is precisely what
  let one shape work in one consumer and not the other; one primitive means
  the next prefix-building shape lands in both at once.
  Also removed while here: `scan_call_facts`'s `command == "call"` literal,
  the last command name hardcoded into that scan. It is now
  `Traits::INVOKES_USER_PROC`, which both fixes a misfire (a user proc
  *named* `call` under a dialect with no such invoker was read as an
  indirection) and generalises to any dialect's equivalent.
  Tests: `interprocedural.rs` gains five cases (every prefix shape, the
  reported `trace add variable … write` shape, a computed-prefix TN control,
  the registry-driven invoker in both directions, and the primitive's own
  shape table); `unit_scope.rs` gains the `trace` registration in both
  bareword and built spellings.
  **Adjacent hole found while measuring, NOT fixed here** (different
  mechanism, deserves its own investigation): a `[cmd …]` substitution in a
  *plain statement's* argument records no call-graph edge at all —
  `puts [pick]` and `lsort -command [pick] …` both yield none, while the
  same substitution in an assignment value (`set x [pick]`) or a `return`
  value does. `scan_value_substitutions` is reached only from
  value/return/expr scanning; the `Statement::Call` arm does not propagate
  into its arguments.

## Confirmed true-positive this audit (sampled, no change needed)

- [x] **W304** missing `--` terminator — tclsh confirms `switch $x` / `file
  delete $f` consume a leading-dash value as an option. TP. (1453)
- [x] **W103** `open` variable arg — tclsh confirms `open "|cmd" r` pipes even
  with an explicit access mode. TP. (398)
- [x] **W212** substitution where var-name expected — `set $x` / `incr $x` /
  `lappend $x` are genuine dynamic-name foot-guns; `upvar`/`dict`/`trace`/
  `namespace which` correctly exempt. TP. (390)  **One FP class fixed
  (FP-STY-12):** the braced indirect-array form `set ${var}(idx)` (where `var`
  holds an array name) is now exempt — it is the deliberate indirect-element
  idiom, not a foot-gun.  Bare `$x` / `$arr(idx)` / index-less `${x}` still fire.
- [x] **W301** uplevel multi-arg concatenation — TP (logger.tcl idioms). (291)
- [x] **W313** destructive op with variable path — TP. (95)
- [x] **W110 / O120** `==`/`!=` on strings → `eq`/`ne` — TP (near-duplicate pair;
  consolidation is a policy call, noted below). (1673 / 1515)
- [x] **W002** disabled-in-dialect — confirmed the `oo::configurable` "FP" was a
  harness artifact (dialect detection); real firings (e.g. `log` disabled) TP.

## Spot-checked, mostly TP (need a fuller sweep to be sure)

- [x] **W211 / W220** — call-by-name suppression extended to cmd-subst
  call sites (see Resolved).  Remaining W211/W220 firings sampled across
  20 large tcllib files audit as genuine vestigial vars (`alpha = asin(r)`
  computed but never used in `mapproj`, `set noskip 1` flag never tested,
  etc.) — no further FP class identified.
- [x] **S102** shimmer (358) — phi-merge shimmer mostly TPs (DES/
  blowfish `set right [expr {$right ^ $cbcright}]` is genuine
  string-build-then-XOR; snit `valcommand` list→string per `uplevel`
  is genuine).  Three FP fixes shipped:
  1. **Hex/binary integer literals** (``0x80``, ``0b1010``) now type
     as INT (was STRING).  Small but real reduction on hex-heavy
     code (idna, DES, AES).
  2. **Destructure-foreach pollution**: ``foreach VARS LIST break``
     (pre-8.5 ``lassign`` equivalent — single-iter foreach used as
     a multi-assign) was contributing its var bindings to the
     function-wide ``loop_body_types`` map.  Detect destructure
     foreach (body block contains only ``IRCall(break)``) and
     exclude its blocks from ``in_loop`` body-type collection.
  3. **Sibling-loop pollution**: the function-wide ``loop_body_types``
     unioned types across ALL loops.  Loop A setting ``\$x`` to STRING
     + loop B setting ``\$x`` to LIST would make S102 fire on EITHER
     loop's phi because the union had ≥2 types.  Per-loop body_types
     (computed via LoopForest natural-loop bodies, keyed by header)
     prevents cross-loop pollution.  Sample tcllib impact: 93 → 30
     (-68%) on 6 high-S102 files; me_cpucore.tcl 48 → 3.
- [~] **W123** unresolved command (1761) — mostly real missing stubs (argparse,
  dget/dexist, custom widget cmds). Not analyser FPs, but a per-package stub
  pass would cut noise. Triage which are stdlib-ish vs project-local.

---

## NOT YET INSPECTED — optimisation hints (O-series)

These drive the optimiser view + quick-fixes; an FP here is a misleading "you
can simplify this" suggestion. None swept yet.

- [x] **O110** Canonicalise expr (InstCombine) — RESOLVED (see top):
  four passes (whitespace guard ×2, bitwise/shift paren-preservation,
  commutative-reorder suppression).  Heavy-bitwise corpus sample:
  389 → 108 (−72%).
- [x] **O116** Fold constant list command — RESOLVED (see top): empty
  `[list]` fold now produces `{}` (was empty string → broke apply).
- [x] **O109 / O126** — RESOLVED (see top): call-by-name suppression
  extended to optimiser DCE (matches W211/W220).
- [x] **O106** Hoist loop-invariant computation — RESOLVED (see top):
  purity check recurses into inner command substitutions.
- [ ] **O120** use eq/ne (1515) — pairs with W110; check the dup-with-W110 policy.
- [ ] **O100** propagate constant into arg (349)
- [x] **O116** fold constant list command — RESOLVED (see top):
  `[list]` empty fold now produces `{}` (apply-correctness bug fixed).
- [ ] **O105** (300)
- [ ] **O127** remove inlined assignment (496) — sampled and audited:
  HINT-level store-to-load forwarding suggestion; the named
  intermediates are stylistic.  Could fire on user-named clarity
  variables — left as HINT only.
- [x] **O126** remove unused variable assignment — RESOLVED (see top):
  call-by-name suppression mirrors W211/W220 (also extended to
  cmd-subst sites).
- [ ] **O111** brace expression text (219) — sampled: all firings on
  unbraced `expr`/control-flow conditions; confirmed TP per Tcl spec.
- [ ] **O101** fold constant expression (205) — sampled: real fold
  opportunities; TP.
- [ ] **O112** (199) — sampled: SCCP-driven dead-`if` elimination.  TP.
- [x] **O109** eliminate dead store — RESOLVED (see top): call-by-name
  suppression on both the analyser (W220) and the optimiser sides.
- [x] **O106** Hoist loop-invariant computation — RESOLVED (see top):
  purity check recurses into command substitutions.
- [ ] **O107** eliminate unreachable code (116) — RCH family has FP tests; re-sweep.
- [ ] **O125** (0 corpus) — verify it can still fire; synthetic test.

## NOT YET INSPECTED — style / lexical warnings

- [ ] **W111** line too long (36012) — pure length; low FP risk but confirm the
  length config + tab handling. Likely "no change".
- [ ] **W112** trailing whitespace (15609) — pure lexical; likely "no change".
- [ ] **W100** unbraced expr (219)
- [x] **W104** string-concat list building → lappend — RESOLVED (see top): usage/
  template notation suppressed; corpus 165→144.
- [x] **W105** unbraced code-block arg (396) — RESOLVED (FP-STY-14): a body
  argument that is a *single bare variable substitution* (`eval $cmd`,
  `proc $n $a $body`, `namespace eval :: $state(-command)`, `after 0 $coroName`,
  `interp eval $child $contents`) is a script-valued reference, not an inline
  block — bracing it (`eval {$cmd}`) evaluates the literal text and errors.
  Suppressed via `_word_is_single_var`; the eval-injection risk stays with
  W101 and the dynamic-dispatch risk with W307 (which already accepts the
  callback form).  Composite / quoted interpolated bodies (`eval "do $script"`,
  `eval $cmd$args`, `${t}--Coro`) still fire at Error severity.
- [ ] **W106** dangerous unbraced switch body (0 corpus) — synthetic verify.
- [ ] **W108** non-ASCII in token (1)
- [x] **W113** proc shadows builtin (95) — RESOLVED (FP-STY-13): redefining an
  overridable Tcl *library* proc (`unknown`, `history`, `auto_*`,
  `tcl_findLibrary`, `pkg_mkIndex`, `tcl_*WordBreak*` …) is not shadowing a C
  built-in — these are script-defined and documented as user-replaceable, and
  Tcl's own library is what `proc`s them.  Added `_OVERRIDABLE_LIBRARY_PROCS`
  exempt set; genuine C commands (`set`/`clock`/`after`/`socket`/`glob`) still
  fire.  Namespace-qualified shadowing was already exempt.
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
- [x] **W216** broken brace-form array ref `${arr}(x)` — RESOLVED (FP-STY-12):
  in a *variable-name* position (`set`/`unset`/`incr`/`append`/`lappend`/
  `info exists`/`vwait` target) `${var}(idx)` is the legitimate indirect-array-
  element idiom (`var` holds the array name — Tcl's `http` package, 25 firings
  in `http.tcl`), not a broken `$var(idx)`.  Suppressed there; value-position
  `puts ${arr}(x)` still fires.  Same idiom also cleared a paired **W212**
  false positive (`check_name_vs_value` skips the braced indirect form).
- [ ] **W240** constant-false loop condition (0) — synthetic verify.
- [ ] **W241** provably-infinite loop (0) — synthetic; intentional `while 1`
  must NOT fire (known idiom).
- [ ] **W242** loop termination unprovable (27) — sampled; sweep for
  cmd-sub-condition loops that DO terminate.

## NOT YET INSPECTED — security warnings (W3xx) + taint (Txx)

- [x] **W201** manual path concatenation → `[file join]` — RESOLVED for the
  literal-whitespace class (FP-STY-16): every multi-word quoted string that
  merely contains a `/` (HTTP request line `"CONNECT $host:$port HTTP/1.1"`,
  usage message `"Usage: … script "`) was flagged.  Rendered-props pass now
  sets `HAS_LITERAL_SPACE` on a top-level `SEP` token and W201 skips it; genuine
  path concat (`"$dir/$name"`, `"$dir/[file tail $path]"`) still fires.  Known
  residual: bracketless CIDR `"$ip/$mask"` / HTML `src=$a/$b` (no literal
  whitespace to key on).  (W201 is a taint-pass diagnostic — not surfaced
  through the server push path — so it is pytest-covered only.)
- [x] **W302** catch without result var — RESOLVED (see top): exempted
  the documented fire-and-forget idiom (`catch {after cancel}`, `catch
  {file delete}`, `catch {close}`, etc.).
- [ ] **W303** ReDoS regexp (0 corpus) — synthetic verify.
- [x] **W307** non-literal command name — RESOLVED (see top): proc-
  param dispatcher + multi-dispatch local heuristic, with taint guard
  to keep firing on tainted dispatch (security correctness).  Cross-
  proc object provenance (factory return-type tracking through
  interproc summaries) remains as a follow-on for the smaller
  residual.
- [ ] **W308** subst without -nocommands (0 corpus) — synthetic.
- [ ] **W309** eval/uplevel with subst (0 corpus) — synthetic.
- [x] **T100** tainted → code-exec sink — RESOLVED (see top): direct-
  operand expr filter; tainted vars inside command substitutions
  consumed by inner commands no longer flag.
- [x] **T101** tainted → output sink — RESOLVED (see top): position
  filter on `puts` (channel arg vs output string).
- [x] **T102** tainted in option position (2) — INJ family has
  position-aware fix; verified.

## NOT YET INSPECTED — errors (Exxx) + hints

- [~] **E001** missing subcommand / **E002** too few args / **E003** too many
  args — arity. Sweep for custom-arity commands (ensembles, varargs) that may
  miscount. (E002/E003 fire in corpus.)  **One lexer-root-cause FP class fixed
  (FP-STY-15):** every corpus E002/E205 firing (tcltest.tcl, csv.tcl) was a
  quoted word ending in the regex end-anchor `$"` (`regsub "\n$" …`,
  `string match "abc$" …`) — the lexer misread the closing quote as a new
  opening quote and merged the word with the next, dropping an argument.  Also
  cleared the paired W306 FP on `"^foo$"` end-anchors (the `$` is literal, not
  a substitution).  Genuine `$bar` foot-guns in quoted patterns still fire W306.
  **2026-07-10 deep review — four false-negative gaps closed** (custom-arity
  commands the sweep above called out, plus one adjacent registry gap found
  alongside them): (1) `TclOO` constructor calls (`ClassName new ?args?` /
  `ClassName create name ?args?`) were never arity-checked at all — now
  resolved against the nearest explicit `constructor` in the class's MRO
  (`ClassHierarchy::constructor_provider`), abstaining when no class in the
  hierarchy declares one (TclOO's inherited default constructor accepts any
  argument count). (2) A direct `apply {{params} body} ?args?` lambda call
  was likewise unchecked — now checked against the lambda's own parsed
  parameter list. (3) `namespace ensemble create -command NAME` (the
  explicit-name form) wasn't recorded as a known command at all — only the
  implicit "same name as the enclosing namespace" form was — causing a
  spurious W123 (and wrong-reason arity abstention) on every call through it.
  (4) `after ms script` / `after idle script` carried no `ArgRole::Body` at
  all (unlike `fileevent`/`chan event`'s identical zero-appended-args shape),
  so a bareword callback's own arity was invisible to every diagnostic path.
  See `kcs-diagnostic-e002-too-few-arguments.md` / `-e003-` for the updated
  user-facing scope list.
  **2026-07-10 follow-up — all five items closed in this pass:**
  1. `next`/`nextto` call arity — wired the existing, already-tested
     `ClassHierarchy::next_provider` into a new registry-driven queue/flush
     pair (`Analyser::queue_next_arity_candidate` /
     `flush_next_arity_diagnostics`), dispatched off a new
     `Traits::TCLOO_NEXT_CHAIN` bit on `next`/`nextto` (not a name check) and
     `Analyser::current_method_context` (which class/method body the call
     textually sits in — reset across a nested `proc`, since `next` only
     resolves inside the method's own calling frame). Treats the enclosing
     method's own class as the receiver's MRO — exact for single
     inheritance; a known, narrow imprecision with mixins/multiple
     inheritance (documented at the flush site), in the same spirit as this
     file's other accepted gaps.
  2. `dict create`/`dict replace`/`dict update`/`foreach`/`switch`'s
     even/odd-count shapes — `Arity` gained `step` (an arithmetic-progression
     constraint) and `also_exact` (a single exception count, for `switch`'s
     shorthand-or-pairs union), both defaulting to "no constraint" so every
     pre-existing `Arity` call site is unaffected. A new **E005** diagnostic
     reports an in-range count that doesn't fit the shape. All five commands'
     derivations verified against a real tclsh 8.6.14. See
     `kcs-diagnostic-e005-wrong-argument-count-shape.md`.
  3. `rename`/alias re-established after deletion — `deleted_commands`
     previously stored one offset per name and any later call was compared
     only against *that* offset relative to the *call site*.
     `resolve_indirect_call_target` now compares a deletion's offset against
     the specific fact's (proc def / rename hop / alias) own establishing
     offset (`fact_superseded_by_deletion`), so a name re-established after
     an earlier deletion resolves to the new definition instead of being
     treated as permanently gone — timestamp-compared, as originally
     planned. `interp alias`'s query-vs-deletion distinction from the
     previous pass is untouched.
  4. `xcDiagnostics` mislabeling / accidental general-purpose gating — split
     into two independent toggles: `xcDiagnostics` (unchanged — the
     f5-irules-only XC100-301 translatability lints) and a new
     `crossFileResolution` (cross-file W120/W123 suppression + cross-file
     E002/E003/E005 arity, every dialect), both default **off**. A plain Tcl
     project can now opt into cross-file analysis without also opting into
     an unrelated F5 migration feature. Existing reschedule/refresh plumbing
     (`reschedule_all_open_documents` et al.) already covers the new toggle
     unconditionally, so no extra wiring was needed there. VS Code exposes
     `tclLsp.features.crossFileResolution`; JetBrains/Neovim/Emacs/Helix/
     Sublime can already set it via `[features] crossFileResolution = true`
     in `.tcl-lsp.ini`/`config.ini` (generic `[features]` key parsing), but
     a dedicated JetBrains settings-panel checkbox is a follow-up (the other
     editors have no per-setting UI to begin with).
  5. Braced multi-word command-prefix (`-command {cb extra}`) — no longer
     silently dropped: `command_prefix.rs` now list-parses a braced prefix
     via `tcl_syntax::list::find_element` (the canonical `Tcl_SplitList`
     primitive, already used elsewhere for proc param lists — reused, not
     reimplemented), records the baked argument count on
     `SignatureCommandInvocation` via a new dedicated `callback_baked_args`
     field (kept separate from the legacy direct-call `argc` field, which
     must stay `None` for a callback head so it doesn't also trip the
     unrelated cross-file direct-call arity path), and
     `apply_callback_arity`'s existing `baked + appended` check — already
     generic — now actually exercises the braced-prefix path for the first
     time. Also closed a Tk widget-path false positive this surfaced: a
     `.widget` head (e.g. `-yscrollcommand {.sb set}`) is never a checkable
     command reference in either prefix shape, reusing the registry's own
     `tk_checks::is_widget_path` rather than a new ad-hoc check.
  **2026-07-10 follow-up — two centralization gaps in the above closed:**
  (1) the `new`/`create` constructor-arity check missed `createWithNamespace`
  entirely (`ClassName createWithNamespace name ::ns ?args?` — the three
  keywords' word layout is shared with `oo_class_arg_roles`'s identical
  class-*definition* shapes) — `PendingCtorArity`'s `is_create: bool` is now
  `CtorForm::{New,Create,CreateWithNamespace}`, each with its own leading-
  word bump. (2) `handle_namespace_ensemble`'s `-command` extraction scanned
  every word for literal equality with `-command`, so another option's value
  word that happened to read `-command` (e.g. a pathological `-map` value)
  could be misread as the flag itself — `namespace ensemble create`'s option
  surface (`-command`/`-map`/`-parameters`/`-prefixes`/`-subcommands`/
  `-unknown`, verified against this project's own `cmd_namespace.rs` VM
  implementation) is now registry data (`ENSEMBLE_CREATE_OPTIONS`), walked
  by declared value arity like every other option-skip in the analyser.
- [x] **E004** malformed `if` — RESOLVED (0 corpus firings; verified against
  Tcl 9.0.4's `TclNRIfObjCmd`/`IfConditionCallback` C source and tclsh 8.6
  rather than a corpus sweep, since real-world malformed `if`s don't occur):
  cleared a genuine FP — `if else {a}` / `if elseif {a}` are structurally
  well-formed (the bareword sits in the condition slot, never keyword-matched
  there; real Tcl fails at expression evaluation instead, a distinct problem).
  Also fixed a redundant-diagnostic bug (a malformed `if` drew both a generic
  E002 and E004 for the same defect — `if`'s registry arity floor is now
  descriptive-only, gated by `Traits::STRUCTURALLY_CHECKED_ARITY`), replaced
  the generic "Malformed 'if' command" message with Tcl's own precise wording
  per sub-case, and narrowed every span from the whole statement to the
  offending word(s). The clause-chain walk itself moved into `tcl-registry`
  (`commands::tcl::if_::walk_if`, exposed as a `clause_shape_check` hook on
  `CommandSpec`) so it is shared with the `if_arg_roles` highlighting
  resolver instead of being re-implemented in the compiler.
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
