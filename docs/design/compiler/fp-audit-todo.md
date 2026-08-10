# False-positive audit — full code inventory & todo

A living checklist for sweeping **every** diagnostic / optimisation / shimmer
code against the real corpus (tcllib 2.0, Tcl 9.0.3 stdlib, tklib 0.9, tdom,
**SpiceGenTcl**) looking for false positives.

**Method per code:** dump every firing with a dialect-aware corpus harness (see
the note below), group by site/shape, reduce the highest-volume shapes to a
minimal repro, verify against **C tclsh 9.0.3**, and either (a) fix the FP with
paired TP/FP regression tests, or (b) record "confirmed true-positive / no
change" with the reasoning.

> **The native harness is available.** Python's `bench/fp_snippets.py` was
> retired with the Python product, but `cargo xtask fp-sweep` replaces it. It
> runs the Rust analyser and compiler checks with the same dialect selection as
> the LSP. Use the command shown below rather than reviving a second sweep
> implementation.

> **Harness correctness note (learned this round):** a raw `get_diagnostics(src)`
> uses the default dialect `tcl8.6`. The corpus must be swept *dialect-aware* —
> detect `package require Tcl X.Y` / `# tcl-dialect:` and wrap in
> `dialect_scope(...)`, exactly as the LSP does — or version-gated commands
> (e.g. `oo::configurable`, a Tcl 9 command) produce phantom W002/W004 that are
> NOT real FPs. The audit harness applies this.

Legend: `[x]` inspected & resolved · `[~]` partially inspected · `[ ]` not yet
inspected · counts are dialect-aware corpus firings as of the last sweep.

---

## 2026-08 harness rebuild + resumed sweep (issue #1316)

`bench/fp_snippets.py` no longer exists — `bench/` went away whole with the
Python retirement (AGENTS.md: "Python has been fully retired on this
branch"), and nothing replaced it, so all 40 rows below sat un-actionable.

**Harness, rebuilt native:** `cargo xtask fp-sweep --code CODE [--code
CODE...] --corpus PATH [--corpus PATH...] [--examples N]`
(`rust/xtask/src/fp_sweep.rs`). Reproduces the documented method exactly:

- **Dialect-aware** — resolves each file's dialect with
  `tcl_cli_support::InputDocument::effective_dialect`, the same detector
  (`# tcl-dialect:` / content signal / extension, falling back to `tcl8.6`)
  `tcl diag` and the LSP server use, closing the harness-correctness note
  above natively.
- **Every code the editor can publish, one pass** — runs `Analyser::analyse`
  (W/E/H), `run_all_checks` + `optimise_unit` over one
  interprocedural-summarised unit (O/S/T, mirroring
  `tcl_lsp_db::compiler_check_diagnostics_uncached`, the documented
  no-salsa-input fallback — a superset of `tcl diag`'s own collector, which
  deliberately drops the O-series), and `tcl_lsp_core::source_style`'s
  pure-text pass (W111/W112/W115/W118), so nothing the editor can show is
  invisible to the sweep.
- **Grouped by site shape** — firings are bucketed by a normalised message
  (digit runs / quoted identifiers → a placeholder), highest-volume shape
  first, matching the resolved entries' own style below
  ("corpus 3641 → ~700-900").

**Corpus available in the initial rebuild:** the canonical corpus (tcllib 2.0, Tcl
9.0.3 stdlib, tklib 0.9, tdom, SpiceGenTcl) could not be fetched — this
sandbox's egress policy blocks `codeload.github.com` (403, confirmed via the
agent-proxy's own guidance not to retry/route around a policy denial), the
same constraint noted in this repo's git history for prior sessions. The
sweep instead ran against every `.tcl`/`.irul` file already committed to
this tree: `samples/` (curated single-diagnostic examples + real-shaped
smoke fixtures), `runtime/rust/vendor/tcl_library` (the **vendored, genuine
C-Tcl standard library source** — `init.tcl`, `package.tcl`,
`tcltest/tcltest.tcl`, … — 5.6k real lines, the closest available substitute
for the "Tcl 9.0.3 stdlib" corpus leg), `rust/tcl-irule-test/tcl` (the
hand-written TMM-simulation orchestrator, ~1.7k lines of real, non-trivial
iRules-adjacent Tcl), `scripts/dev/diag_parity/corpus`, and
`editors/vscode/testFixture` (noisier — many fixtures are *deliberately*
malformed to exercise a specific other diagnostic, so a co-firing there was
weighted lower than the same shape in the other four). Every finding below
that names a real file path came from this corpus; every "(0 corpus)" entry
was verified synthetically instead (a minimal repro run through `tcl diag`,
cross-checked against real `tclsh8.6.14` where runtime behaviour was in
question), per the checklist's own stated fallback for those rows.

**Result: two confirmed, fixed false positives**, both with paired FP/TP
regression tests and both verified against real `tclsh8.6.14`:

- **W215** unreachable variable name — `set ns::$k v` (any unbraced
  variable-defining word with a live, non-leading `$var`/`[cmd]` piece) fired
  spuriously. `word_piece` (`tcl-compiler/src/segmenter.rs`) re-braces an
  unbraced substitution piece when reconstructing a multi-token word's
  display text (`$k` → `${k}`, so an adjacent literal suffix can't run into
  it); W215's reachability check was inspecting *that reconstruction*, not
  the source or the runtime name, and flagging its own `{`/`}` artefacts.
  Confirmed against tclsh 8.6.14: `set k client_addr; set ::ns::$k hi`
  writes the perfectly ordinary, `$`-reachable `::ns::client_addr` — nothing
  about it is unreachable. Corpus instance: `rust/tcl-irule-test/tcl/runner.tcl`
  (12 firings, all this shape). Fixed in `emit_w215_unreachable_name`
  (`analyser/scope.rs`): abstain from the name-reachability half of the
  check (the array-element `)` half is unaffected — a dynamic index already
  round-trips verbatim) whenever the *source* word itself carries a `$`/`[`.
- **W242** loop-termination unprovable — `while {[cmd $u] > $rest} { …
  reassigns $u … }` blamed `rest` (a static threshold) instead of
  recognising `u` (which the body visibly shrinks) as the real progress
  variable, because `extract_counter_name` only looks for the first *bare*
  `Variable` token in the condition and a `[cmd $u]` command substitution
  hides `u` from that scan entirely. This is exactly the risk the checklist
  itself flagged sight-unseen ("sweep for cmd-sub-condition loops that DO
  terminate"). Corpus instance: `runtime/rust/vendor/tcl_library/tcltest/tcltest.tcl`
  (2 firings — the option-usage word-wrapper). Fixed in `extract_counter_name`
  (`analyser/bounds_checks.rs`): abstain when the condition contains any
  `[cmd …]` substitution, matching the module's own documented "intentionally
  shallow … avoiding false positives" philosophy.

**Also found and corrected in-place: two stale entries.**

- **W122** — this sweep found 0 firings anywhere, corroborating issue
  #1317's independent finding that W122 has no emitter left in the tree at
  all (superseded by W124, which covers both halves of its description).
  Not re-litigated here; tracked and resolved under #1317.
- **W308** — the checklist's row for this code (below) described a
  `subst`-without-`-nocommands` security check. The registry's own test
  suite (`tcl-core-types/src/diag_code.rs`,
  `w308_documents_the_tcloo_unknown_method_check`) already documents that
  this was a historical mislabel: W308 is the TclOO unknown-method check
  (`$obj badMethod` → "did you mean 'goodMethod'?"), and no emitter for the
  old meaning ever existed (that hazard is W102 / the T100 taint sink gate).
  Verified the current check fires correctly on a real unknown-method call;
  the row below is corrected to match what the code actually does.

**Audited this session, no defect found** (synthetic and/or corpus,
verified against `tclsh8.6.14` where runtime semantics were load-bearing):
W100, W106, W108, W111, W112, W114, W116, W117, W118, W120, W121, W124,
W125, W127, W213, W240, W241, W303, W308 (corrected meaning), W309, E200,
H300, TK1001, IRULE1001, IRULE1005, O100, O101, O105, O107, O112, O120,
O125 (real corpus firings found — see its row; the shape is plausible but
not deep-audited), O127. Each row below carries the specific evidence.

**Superseded by the follow-up below:** the initial run did not reach the
`IRULE1002`–`5007` family beyond IRULE1001/IRULE1005. The later dedicated
iRules sweep covers that gap. TK1002/TK1003 were spot-checked only by family
resemblance to TK1001 (verified), not individually run. The three
cross-cutting follow-ups at the end of the file are policy decisions, not
per-code sweeps, and are annotated with a recommendation rather than closed
outright.

---

## 2026-08 follow-up: iRules lifecycle and residual arity audit

The native `cargo xtask fp-sweep` harness had already landed on `rust` for
#1316. This follow-up uses that harness to complete the iRules and residual
audit work; it does not introduce a second sweep implementation.

The native sweep was re-run over 304 committed Tcl sources for the formerly
unswept E001/E002/E003 group, and over 204 runnable iRules files from seven
public example repositories fetched by `scripts/dev/fetch-irules-corpus.sh`.
The iRules run produced 837 findings. Counts are triage leads, not proof: an
iRule can depend on a virtual server profile, traffic direction, or another
rule that is not present in one file, so every change below has a reduced
reproducer and registry-level regression test.

- **IRULE1004 (297 findings)** was an unconditional false positive. BIG-IP
  accepts `when EVENT { ... }` and supplies priority 500. The registry now
  records that default and the analyser abstains unless a dialect's
  event-handler policy explicitly requires a priority. An explicit priority
  remains useful when a deployment needs a deliberate rule order.
- **IRULE1006 (four findings)** used a `*::payload` spelling rule. That
  incorrectly required a non-existent `UDP::collect` and an ASM collect step.
  Payload availability is now declared per command in the command registry.
  HTTP, TCP, SSL, MQTT, MR, RTSP, SCTP, and WebSocket declare their collection
  lifecycles, including explicit versus data-event release. Payload supplied
  by ASM, CACHE, DIAMETER, GTP, REWRITE, SIP, UDP, and XML events is explicitly
  classified as immediately available. MQTT also declares its call-form
  split: bare and `append` need collection, while `length`, `replace`, and
  `prepend` operate on the current PUBLISH message.
- **IRULE1007 (four findings)** required `HTTP::release` after every
  `HTTP::collect`. BIG-IP implicitly releases HTTP data at the matching data
  event unless a new collect starts. That release policy is registry data and
  the cross-event checker consumes it generically. Event execution sides and
  nested `clientside`, `serverside`, and `peer` bodies are registry facts too,
  so the checker does not infer lifecycle state from event or command names.
  TCP and SSL still require an explicit release.
- **E003 `source -nopkg`** is a Tcl 9 flag used by the vendored Tcl 9 library.
  The command spec now gates it to Tcl 9.0+. C Tcl 9 also accepts the lower-case
  `package require tcl 9.0.3` spelling used by its own `init.tcl`; dialect
  detection recognises that exact Tcl-9-only alias without treating arbitrary
  package-name casing as a match. A standalone extracted Tcl 9 file with no
  directive, shebang, or version guard still defaults to Tcl 8.6 by design, so
  add `# tcl-dialect: tcl9.0` in that case.
- **E001 in `tcltest.tcl`** is a remaining conservative limitation, not a
  proved command-arity defect: tcltest creates a command through a dynamically
  computed `proc` name. The runtime command exists after initialisation, but a
  static call graph cannot prove the name without executing package setup.

A final ten-code residual sweep over a fresh 206-file corpus from eight of the
nine repositories completed without a crash. IRULE1002/1004/1005/1008/1201
had no firings. The remaining findings were inspected at source:

- **IRULE1003 (one)** is a genuine deprecated `ASM_REQUEST_VIOLATION` event.
- **IRULE1006 (one)** reads `HTTP::payload` without any `HTTP::collect`.
- **IRULE1007 (one)** collects HTTP data, but places `HTTP::release` under a
  newline-separated `else` command rather than the Tcl `if` command; that file
  is invalid Tcl and has no executable release path.
- **IRULE1202 (eight unique ranges)** comprises six in one OAuth example and
  two in a JavaScript-challenge example. The OAuth rule calls `HTTP::respond`
  on continuing paths; its `event disable all` calls disable later event
  processing but do not return from the current handler. The challenge rule's
  first conditional response also lacks a return, so either of its later
  responses can run on the same request. Multiple predecessor proofs
  previously repeated the same range; the flow consumer now emits one
  diagnostic per code, range, and message.
- **IRULE4004** initially produced 74 findings. Most were request resets,
  conditional assignments, or variables written again on another path; moving
  them changed connection state after the first request. The check now requires
  an unconditional entry-block literal scalar and its sole write across every
  event in the rule. The re-sweep leaves nine genuine configuration literals
  that can move to a once-per-connection event.

The sweep also found advisory-heavy families that need configuration-aware
evidence before they can be changed safely (taint warnings, variable scope
checks, logging, and top-level event-context calls). They are
not declared true or false from corpus count alone.

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
  `-command` callback options, issue #978); `uplevel #0` body namespace
  context (issue #980).
  **`namespace ensemble … -map` redirection (issue #979)** — investigated
  and closed as *already sound, bluntly*: the scan has no model of ensemble
  dispatch, but the registry marks `namespace ensemble` with
  `Traits::EXPORTS_COMMAND`, which declines whole-module seeding, so a proc
  reached only through a `-map` target never folds.  tclsh8.6/9.0-confirmed
  that `namespace ensemble create -command myens -map {go helper}` +
  `myens go dev` really does reach `helper` with `"dev"`.  Nothing pinned
  that, so a registry edit narrowing the trait — or a precision follow-up
  that starts resolving *some* ensemble maps — would silently reopen
  FP-IPCP-01's exact shape; `ensemble_map_redirected_caller_does_not_fold_issue_979`
  now guards the mechanism, whatever replaces it.  Resolving ensemble maps
  precisely (so an *agreeing* mapped caller could still fold) remains an
  optional precision follow-up, not a soundness gap.
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
  **Newly confirmed by the same review, since CLOSED (issue #980):**
  `uplevel #0 { … }` also resolves against global (tclsh8.6/9.0-confirmed),
  and reproduced the identical misattribution + phantom-fold pair as the
  `TclOO`/namespace-eval cases, but through two distinct routes. The scan
  never reached that body as an `UpFrame` statement at all —
  `Statement::UpFrame` keeps its body as a nested `Script` that CFG
  construction does *not* flatten into blocks, so the walk over
  `block.statements` skips it — while the *enclosing* `proc` statement's own
  `ArgRole::Body` argument was re-segmented in the frame that `proc`
  statement sits in, inventing a call to a same-named proc in the declaring
  namespace. Both halves are fixed:
  - `build_extra_call_site_scan_contexts` now builds a bare CFG for every
    static `uplevel` body (`upframe_scan_bodies`). `frame_shift == 0`
    resolves as `"::top"` — the same global-resolving pseudo-qname a method
    body is forced to — while a relative level keeps the enclosing unit's
    namespace. Each gets a synthetic occurrence-unique CFG name so its
    variable-scope facts never clobber another scope's.
  - The `ArgRole::Body` recursion no longer re-walks a body the registry
    marks `Traits::DEFINES_PROCEDURE`. A definition body does not run at the
    definition site, and lowering already registers it as a procedure /
    method / body unit the scan visits with the right context — the same
    rule the `namespace eval ::abs { … }` guard beside it already applies,
    generalised from "absolute `ArgRole::Name`" to the registry's own
    "this command defines a procedure" fact.
  `uplevel N` for any relative (non-`#0`) level remains a separate,
  permanent approximation: the target frame's namespace depends on the live
  call stack, which is undecidable by a single-file static analysis — now
  pinned by `uplevel_relative_body_keeps_the_enclosing_units_namespace`
  rather than left implicit. Tests: the pinned
  `uplevel_zero_body_resolves_against_global_not_enclosing_namespace` is
  un-ignored and passes; new
  `conditionally_defined_proc_body_call_sites_are_still_counted` guards the
  definition-body skip against losing a real call site; plus the
  `namespace_eval_body_unit_does_not_change_bytecode` regression in
  `regex_source.rs` updated for the corrected qname format.
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

- [x] **FP-IPCP-04** Composing an unenumerable dispatch with cross-file
  evidence — CLOSED.
  FP-IPCP-02 recorded "a caller exists that names no callee I can identify"
  (`set cmd [gets stdin]; $cmd dev`, `eval $script`, `apply $fn`) as one
  module-wide flag, which is right within a unit. FP-IPCP-03 then merges
  several files' evidence into one project view — and a flag has no callee to
  narrow by, so it spread: one `eval $script` anywhere in a workspace
  withdrew every interprocedural seed in every file, costing three true
  positives outright in the extension suite.
  The fix is neither to drop the propagation (unsound) nor to keep it
  (useless) but to bound what such a dispatch can **reach**, and to record it
  *per callee* over that set so it composes through `merge_from` and
  `slice_for` with no special casing. The module-wide gate in
  `params_constants_from_call_sites` then disappears entirely: a withdrawn
  seed is just a poisoned slot, which `uniform_literal_at` already handles.
  **The bound is the `source`-connected component, and it took two attempts.**
  The first tried to derive it inside the compiler from the scanning file's
  own linkage traits — "a file that pulls in no other unit cannot dispatch
  outside its own declarations". That is right for the *outbound* direction
  and wrong for the inbound one: a library declaring no `source` of its own
  still runs inside its sourcer's interpreter, so its unreadable dispatch can
  name the sourcer's procedures. Caught by probe, not by the suite —
  `main.tcl` sources `lib.tcl`, `lib.tcl` has `set cmd [gets stdin]; $cmd
  dev`, and `main.tcl`'s `helper` still folded.
  Connectivity is a *project* fact, so the host owns it:
  `tcl_lsp_db::file_link_targets` records each file's resolved literal
  `source` targets (signature-level, so a body edit backdates), and
  `file_dispatch_reach` unions the procedures of every file in the same
  **undirected** component — undirected precisely because the exposure runs
  both ways. `unit_scope::scan_source_call_sites` takes that set from the
  host rather than deriving it; with none supplied the file's own
  declarations remain the honest bound, and within a single unit the reach is
  that unit's own procedures, so in-unit behaviour is exactly what FP-IPCP-02
  specified. `tcl diag a.tcl b.tcl` supplies the union of its inputs — naming
  files on one command line asserts they are one program.
  Only *literal* `source` targets create edges: guessing a computed path
  would widen what an unenumerable dispatch may reach, so a missed edge is
  the safer error.
  Tests: five in `unit_scope` (own-unit withdrawal; reaches a linked file in
  both directions; leaves an unlinked one alone) and two end-to-end in
  `tcl-lsp-db` over a real two-file project with paths.

- [x] **FP-IPCP-05** I230 on a module's own `unknown` handler — CLOSED
  (issue #1044).  The last caller class the scan structurally could not
  enumerate: Tcl dispatches *every* command word that resolves to nothing to
  the interpreter's unresolved-command handler, passing the word itself
  followed by that call's own arguments.  A module defining `proc unknown
  {cmd args}` was therefore seeded from its *direct* callers alone, so
  `unknown alpha` twice plus a `bogus beta` anywhere in the file bound `cmd`
  to the constant `"alpha"` and folded `$cmd eq "alpha"` on a condition that
  genuinely varies.  tclsh8.6/9.0-confirmed before fixing: `bogus beta gamma`
  runs the handler with `cmd` = `bogus`, `args` = `beta gamma`; and a
  namespace-local `proc unknown` is *not* consulted for unresolved words in
  that namespace — `::unknown` handles them regardless of calling namespace,
  so the lookup is global-scope only.
  Which command is the handler is registry data, never a literal in the
  compiler: a new `Traits::UNRESOLVED_COMMAND_HANDLER` (`declare_traits!`
  bit, room already available after #1031's `u128` widening) is carried by
  the `unknown` spec, and `unit_scope::unresolved_command_handler` resolves
  it against the unit's — or, cross-file, the project's — procedure set.
  When `resolve_target` finds no callee for a literal word that the registry
  also does not know, `record_unresolved_word_dispatch` records the handler's
  invocation with `(word, args…)` through the same `record_invocation` path
  everything else uses, so the evidence union is exactly the values the
  handler's parameters really see rather than a blanket poison.
  **Superseded in part — see MC-UNKNOWN-SEED below.** This entry originally
  claimed the invented-dispatch residue "can only *retract* a fold, never
  manufacture one". That was false while those dispatches were the handler's
  *only* recorded call sites, and the adversarial review demonstrated the
  miscompile. The handler is now poisoned unconditionally and the recorded
  dispatches are extra evidence only, which is what finally makes the claim
  true.
  Tests: nine in `unit_scope` (the repro; the word's own arguments following
  it; no-handler module unaffected; a registry builtin is not an unresolved
  word; an unenumerable dispatch still poisons rather than naming the
  handler; plus the four listed under MC-UNKNOWN-SEED) and one on the
  registry's single-carrier invariant, plus three end-to-end in
  `diagnostics::unresolved_word_reaching_the_unknown_handler_does_not_fire_i230`
  and its TP/TN controls.

## Adversarial review of the #1044/#1015/#1013 round — CLOSED

Every item below was confirmed against tclsh8.6 and tclsh9.0 before being
fixed; the probe scripts are named where they exist.

- [x] **MC-UPLEVEL-ZERO** `uplevel 0` miscompiled as `uplevel #0`.
  `parse_uplevel_level` folded the absolute `#N` form and the relative `N`
  form into one `frame_shift`, so `#0` and `0` both reached
  `Statement::UpFrame` as `0` and `upframe_scan_bodies` resolved both
  against the global namespace. They are different frames: inside
  `::foo::runIt`, `uplevel #0 { helper b }` calls `::helper` while `uplevel
  0 { helper c }` calls `::foo::helper`. Every `uplevel 0` body was therefore
  resolving its bare command words globally, sending evidence to a proc that
  is never called and withholding it from the one that is. `UpFrame` now
  carries an explicit `absolute: bool`; the scan resolves globally only for
  an absolute shift of 0, and the relative form takes the enclosing-unit
  branch — which for `uplevel 0` is exact, not an approximation.
  `inline_uplevel`'s passthrough detector gained the matching guard so
  `uplevel #1` is not confusable with `uplevel 1`.
- [x] **MC-UNKNOWN-SEED** The unresolved-command handler's invented
  dispatches manufactured folds. Naming some of an unenumerable set is not
  enumerating it, and the recorded words are wrong in *both* directions:
  `Dog new` after `oo::class create Dog` and `worker` after `coroutine
  worker body` are recorded yet never reach the handler, while a `bogus
  beta` written *before* `proc unknown` is handled by the builtin
  `::unknown` and errors. `scan_cfg_callers` now poisons the handler
  unconditionally the moment the module defines one, before reading a
  statement — which, being shared, also closes the cross-file case where
  one file's dispatch seeded a handler another file defines. The
  concrete dispatches remain as retracting evidence.
  `unresolved_command_handler`'s candidate list is sorted (it came off a
  hash-map walk) and the single-carrier invariant is pinned.
  Tests: `a_handler_never_seeds_even_when_every_visible_caller_agrees_1044`
  (which previously asserted the manufactured fold, on the disproved premise
  that the visible callers were all of them),
  `a_class_command_never_seeds_the_handler_1044`,
  `a_coroutine_command_never_seeds_the_handler_1044`,
  `a_word_written_before_the_handler_never_seeds_it_1044`,
  `a_cross_file_dispatch_never_seeds_another_files_handler_1044`.
- [x] **MC-APPLY-NS** `apply {params body ns}` resolved its body globally.
  `lower_apply` computed the pinned namespace and lowered the body against
  it, then registered the body unit under the bare `apply` marker, whose
  qname puts it in the global namespace. tclsh8.6/9.0: inside `::foo::runIt`,
  `apply {{x} { helper $x } ::foo} b` calls `::foo::helper`, while the
  two-element form calls `::helper`. Fixed by qualifying the marker exactly
  as `lower_namespace_eval` does; `join_namespace` normalises the unpinned
  case back, so the two-element form is untouched.
- [x] **RE-DEAD-EDGE** `compute_reachable_call_offsets` accepted statically
  dead proof edges. A call site's presence in a body is not proof the body
  executes it, but a `proc a {} { if {0} { b } }` edge still handed `b` —
  and transitively everything `b` calls — `a`'s early offset, which read as
  "reached before the deletion" and withdrew a correct W123
  (`review-probes-sound/r1.tcl` really does fail). The precise rule would
  drop edges in SCCP-proved-unreachable regions, but SCCP runs later over
  the IR this analyser's own result feeds, so consulting it here is
  circular. Implemented instead: **a body edge may add a callee offset but
  never lower a base top-level one**. A callee with its own top-level call
  site has an offset that is already an observation of real execution; one
  without keeps the old optimism, which is what #1015's chains need. Still
  misses a dead edge to a callee never called at top level, and can raise
  an offset for a live edge whose callee also has a later top-level call —
  both documented at the function.
  Also documents the pre-existing **earliest-offset false negative** on
  `reachable_call_offsets` (`review-probes-sound/r3.tcl`): one offset per
  name means a proc called both before and after a deletion reads as
  reached-before for all of its invocations. Widened by #1015, not
  introduced by it, and unchanged by design.
- [x] **FP-OBJ-1013R** Class liveness gated at the wrong point, and on two
  wrong facts.
  1. `aggregate_object_types` dropped a class's type when the class was dead
     at *file end*, so a class used before a *later* deletion lost its object
     typing entirely — the dispatch lost its W308 and drew a spurious W307
     instead (`review-probes-sound/w308d.tcl`, which really does fail with
     `unknown method "fly"`). The map is now unfiltered and the gate moved
     to the emit site, which knows the dispatch offset — the same
     granularity #1010 used for the constructor sites.
  2. `rename Dog Cat` was read as a pure deletion of the class. It removes
     the command *name* but not the class, and objects already built from it
     still answer their methods. Class liveness now treats a class named as
     some rename's source as live; the command-name question is separate and
     untouched, so `Dog new` still draws W123.
  3. A `rename` inside a control-flow body counted as an unconditional
     deletion, so `if {0} { rename Dog {} }` flagged `Dog new` with W123 —
     while the analyser proved that very branch dead in the same run,
     emitting I230 on it. Only straight-line renames are recorded now,
     decided by a registry-driven `control_flow_body_depth`
     (`Traits::CONTROL_FLOW` bodies may run zero times or many;
     `namespace eval` / `eval` / `uplevel` bodies always run). This is the
     narrow *syntactic* rule — `if {1} { rename Dog {} }` is equally not
     recorded — matching the existing "a deletion inside a definition body
     does not count" rule.
- [x] **FP-INTERP-CREATE** `interp create`'s flag scan stopped at the path,
  so `interp create x -safe` recorded `x` as unsafe. C Tcl examines every
  word and treats a `-` word as a flag until `--`; tclsh8.6/9.0 confirm
  `interp create n -bogus` errors `bad option`, proving flags are still
  parsed after the path. The scan now mirrors that loop, and a second path
  word — the `wrong # args` shape — records no path rather than an arbitrary
  one. Separately, `interp_create_words_from_value` ended its element scan on
  a parse error and kept the prefix, so `[interp create {child]` read as a
  bare `interp create` and bound the variable to a phantom interpreter;
  a parse error now rejects the whole substitution.
- [x] **FP-UPFRAME-REWALK** A frame-shifting body nested inside another
  `ArgRole::Body` was re-attributed. `proc runIt {} { catch { uplevel #0 {
  helper b } } }` inside `::foo` walks the `catch` body correctly, then
  walked the `uplevel` body again as `::foo`, inventing `::foo::helper`
  alongside the correct `::helper` the upframe scan had already recorded.
  Fixed by a new `Traits::EVALUATES_IN_SHIFTED_FRAME`, stamped on `uplevel`,
  joining the existing `DEFINES_PROCEDURE` skip — so the walker still names
  no commands.
- [x] **VAR-ESCAPE-SCAN-MODE** The three `eval`/`catch` body pre-scans in
  var_escape used `scan_script`, which honours Tcl's word-splitting and so
  misses a name mentioned only inside a brace-quoted word. They now use
  `scan_word`, the over-approximating mode the sites intend. **This is a
  statement of intent, not a behaviour change:** `is_dynamic_token` already
  sends any body containing `$` or `[` down the pessimistic path before the
  scan runs, which is every body the scan could find a reference in, so the
  mode is unobservable today and the guard carries the soundness. Written
  the over-approximating way so narrowing that guard later cannot silently
  reopen the hole.

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
- [x] **O120** use eq/ne — 2026-08 sweep (issue #1316): 7 firings across
  `samples/optimiser/input.tcl`, `runtime/rust/vendor/tcl_library` pkgIndex
  stubs, a real `f5-irules` sample. All genuine `==`/`!=`-on-strings sites;
  TP. See **"W110 / O120 near-duplicate"** below for the still-open policy
  question this pairs with (not a firing-correctness issue).
- [x] **O100** propagate constant into arg — 2026-08 sweep: 69 firings —
  SCCP constant-condition folds on `foreach`/`for`/`while` loop headers
  (mundane and expected: any loop over a literal list/count has a
  trivially-true continuation test) plus real constant-propagation-into-arg
  sites. Spot-checked a representative sample of each shape; all TP.
- [x] **O116** fold constant list command — RESOLVED (see top):
  `[list]` empty fold now produces `{}` (apply-correctness bug fixed).
- [x] **O105** — 2026-08 sweep: 4 firings (`samples/irules/diagnostics_style_perf.irul`,
  `rust/tcl-irule-test/tcl/command_mocks.tcl`, `runtime/rust/vendor/tcl_library/package.tcl`
  + `tcltest.tcl`). Spot-checked the two stdlib instances by hand: both are
  genuine same-expression, same-SSA-inputs recomputations across a
  control-flow join (`tcltest.tcl:2251`'s `[string trim $description]`
  executes unconditionally right after a conditional path that already
  computed it). Plausible TP; not deep-audited beyond these two.
- [x] **O127** remove inlined assignment (496) — sampled and audited:
  HINT-level store-to-load forwarding suggestion; the named
  intermediates are stylistic.  Could fire on user-named clarity
  variables — left as HINT only. 2026-08 sweep: 57 more firings across the
  local corpus, same shape throughout (`Remove inlined assignment` /
  `Inline single-use variable`); no FP found, consistent with the original
  sampling.
- [x] **O126** remove unused variable assignment — RESOLVED (see top):
  call-by-name suppression mirrors W211/W220 (also extended to
  cmd-subst sites).
- [x] **O111** brace expression text (219) — sampled: all firings on
  unbraced `expr`/control-flow conditions; confirmed TP per Tcl spec.
  2026-08 sweep: 0 firings in the available corpus (no unbraced-`expr`
  shapes present); original sampling stands.
- [x] **O101** fold constant expression (205) — sampled: real fold
  opportunities; TP. 2026-08 sweep: 8 more firings, same shape
  (`Fold constant expression` on a literal `expr`); no FP found.
- [x] **O112** (199) — sampled: SCCP-driven dead-`if` elimination.  TP.
  2026-08 sweep: 7 more firings (`Eliminate dead if` / `Eliminate constant
  if` / `Eliminate switch`), each spot-checked against a genuinely
  always-true/false literal condition; no FP found.
- [x] **O109** eliminate dead store — RESOLVED (see top): call-by-name
  suppression on both the analyser (W220) and the optimiser sides.
- [x] **O106** Hoist loop-invariant computation — RESOLVED (see top):
  purity check recurses into command substitutions.
- [x] **O107** eliminate unreachable code (116) — RCH family has FP tests; re-sweep.
  2026-08 sweep: 1 firing (`samples/tcl/02_control_flow_braced.tcl`), a
  genuine unreachable branch after a terminating statement; TP.
- [x] **O125** (0 corpus) — verify it can still fire; synthetic test. 2026-08
  sweep: fires for real — 19 firings (`Sink '…' into branch — prepend in
  target body`) across `runtime/rust/vendor/tcl_library/init.tcl` and
  `rust/tcl-irule-test/tcl`. Spot-checked `init.tcl:437`'s `set f ""`: the
  value is dead on every path (the `if {$issafe}` branch never reads `f`;
  the other branches reassign it before any read), so sinking it is
  behaviourally safe either way — plausible, not a confirmed defect, but the
  "target branch" reasoning around a reassignment inside an `elseif`
  *condition* (not the body) wasn't traced fully; flagged for a closer look
  rather than fixed this session.

## NOT YET INSPECTED — style / lexical warnings

- [x] **W111** line too long (36012) — pure length; low FP risk but confirm the
  length config + tab handling. Likely "no change". 2026-08 sweep: confirmed
  no change needed — 7 firings, all a straightforward `len(line) >
  tclLsp.style.lineLength` count (`source_style::style_diagnostics`, a pure
  text-length check with no Tcl-semantic component to get wrong).
- [x] **W112** trailing whitespace (15609) — pure lexical; likely "no change".
  2026-08 sweep: confirmed — 3 firings, all a literal trailing-`[ \t]+`
  regex match; no FP surface exists.
- [x] **W100** unbraced expr (219) — 2026-08 sweep: 16 firings, all genuine
  unbraced `if`/`expr` conditions (`if $a`, `expr $a + $b`); each risks the
  documented double-substitution / no-bytecompile issue. TP.
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
- [x] **W106** dangerous unbraced switch body (0 corpus) — synthetic verify.
  2026-08: `switch $x a puts_hi b puts_bye` (unbraced pattern/body pairs)
  fires correctly, message matches the real double-substitution risk. TP.
- [x] **W108** non-ASCII in token (1) — 2026-08 sweep: 5 firings, all a
  single em-dash (U+2014) inside a `puts "…"` string literal (2 sample
  files' own prose, 3 in an `.irul` comment-adjacent string). By design
  (module doc, `usage.rs`): only *comments* are exempt from the
  confusables scan — a string literal is flagged the same as code, since
  copy-pasted "smart" typography in a literal is exactly the artifact this
  check exists to catch. TP as designed, not a defect in the sample files.
- [x] **W113** proc shadows builtin (95) — RESOLVED (FP-STY-13): redefining an
  overridable Tcl *library* proc (`unknown`, `history`, `auto_*`,
  `tcl_findLibrary`, `pkg_mkIndex`, `tcl_*WordBreak*` …) is not shadowing a C
  built-in — these are script-defined and documented as user-replaceable, and
  Tcl's own library is what `proc`s them.  Added `_OVERRIDABLE_LIBRARY_PROCS`
  exempt set; genuine C commands (`set`/`clock`/`after`/`socket`/`glob`) still
  fire.  Namespace-qualified shadowing was already exempt.
- [x] **W114** redundant nested `[expr]` (0) — synthetic verify. 2026-08:
  `expr {1 + [expr {$x + 2}]}` fires correctly on the inner redundant
  `[expr]`. TP.
- [x] **W115** backslash-newline in comment (0) — synthetic verify. 2026-08:
  `# comment \` + a continuation line fires correctly
  (`source_style::style_diagnostics`; 0 firings in the local corpus, real
  behaviour confirmed synthetically). TP.
- [x] **W116 / W117** stub shadows builtin command/function (0) — synthetic.
  2026-08: `# tcl-lsp: stub puts {arg} -pure` (W116) and `# tcl-lsp: stub
  expr-func sin 1` (W117) inside a `stubs-begin`/`stubs-end` block both
  fire correctly against real built-ins (`puts`, `sin`). TP.
- [x] **W118** inconsistent line endings (6) — 2026-08 sweep: 0 firings in
  the local corpus (a git checkout normalises line endings, so this
  particular repo's files can't exercise it); not independently
  re-synthesised this session (low risk — a single, file-level
  `\r\n`-vs-`\n` count with no Tcl-semantic component).
- [x] **W120** command without package require (5) — 2026-08 sweep: 11
  firings (`http::geturl`, `snit::type`, `bind`/`winfo`/`text`/`tk`,
  `itcl::class`, `::report::defstyle`), every one a real package-provided
  command used without the matching `package require`. TP.
- [x] **W121** non-contiguous subnet mask bits (0) — synthetic. 2026-08:
  `255.0.255.0` fires with a correct "did you mean '255.0.0.0'?" — the bit
  pattern genuinely isn't contiguous leading-1s. TP.
- [x] **W122** mistyped IPv4 (3) — 2026-08 sweep: 0 firings anywhere in the
  local corpus, corroborating issue #1317's independent finding that W122
  has **no emitter left in the tree at all** (superseded by W124, which
  covers both halves of its description — see the dedup guard at
  `analyser/diagnostics.rs:983`, itself now dead). Not a corpus gap: the
  code cannot fire under any input. Retired under #1317, not re-audited
  here as a live code.
- [x] **W124** invalid IP literal (8) — 2026-08 sweep: 0 firings in the
  local corpus (no malformed IPv4 literals present); synthetic check:
  `999.1.1.1` fires "octet 1 (999) exceeds 255" correctly. TP.
- [x] **W125** orphaned control-flow keyword (0) — synthetic. 2026-08: a
  newline-split `if {1} {...}\nelse {...}` fires correctly ("check for
  misplaced newline") — the textbook Tcl beginner mistake. TP.
- [x] **W126** non-channel value in channel arg — RESOLVED (see top): lassign
  element-type lattice fix; corpus 4→0.
- [x] **W127** value not in allowed set (0 corpus, NEW from #501) — synthetic +
  corpus once a project uses a closed-set command. 2026-08: `interp create
  child; interp limit child bogus` fires "expected one of: commands, time"
  — verified against real `tclsh8.6.14` (`bad limit type "bogus": must be
  commands or time`), byte-for-byte the same closed set. TP.

## NOT YET INSPECTED — variable-shape warnings

- [x] **W213** unset on possibly-unset var (1) — RBS-derived; re-check.
  2026-08: `if {$flag} { set x 1 }; unset x` (conditional def, unconditional
  unset) fires correctly with the `-nocomplain` quick-fix. TP.
- [x] **W215** unreachable variable name (12) — 2026-08 sweep: **FP found
  and fixed** (see the harness-rebuild summary above and "Resolved this
  audit" at the top for the full writeup). `set ns::$k v`-shaped dynamic
  namespace-qualified writes were spuriously flagged; fixed in
  `emit_w215_unreachable_name`, 4 new paired FP/TP tests
  (`analyser/scope.rs`). The remaining corpus firings (array-index `')'`,
  a genuine stray-`}` in `editors/vscode/testFixture`) are real, deliberate
  typo-shaped test fixtures — TP.
- [x] **W216** broken brace-form array ref `${arr}(x)` — RESOLVED (FP-STY-12):
  in a *variable-name* position (`set`/`unset`/`incr`/`append`/`lappend`/
  `info exists`/`vwait` target) `${var}(idx)` is the legitimate indirect-array-
  element idiom (`var` holds the array name — Tcl's `http` package, 25 firings
  in `http.tcl`), not a broken `$var(idx)`.  Suppressed there; value-position
  `puts ${arr}(x)` still fires.  Same idiom also cleared a paired **W212**
  false positive (`check_name_vs_value` skips the braced indirect form).
- [x] **W240** constant-false loop condition (0) — synthetic verify.
  2026-08: `while {0} {…}` fires correctly. TP.
- [x] **W241** provably-infinite loop (0) — synthetic; intentional `while 1`
  must NOT fire (known idiom). 2026-08: confirmed both halves —
  `while {1} {puts loop}` (no exit) fires; `while {1} {if {…} {break}; …}`
  (the documented idiom) correctly does not. TP / TN both hold.
- [x] **W242** loop termination unprovable (27) — sampled; sweep for
  cmd-sub-condition loops that DO terminate. 2026-08 sweep: **exactly this
  — FP found and fixed** (see the harness-rebuild summary above).
  `while {[string length $u] > $rest} { … reassigns $u … }` blamed the
  static `rest` instead of the real progress variable `u`, hidden inside
  the `[string length $u]` command substitution `extract_counter_name`
  can't see into. Corpus instance:
  `runtime/rust/vendor/tcl_library/tcltest/tcltest.tcl` (both of the
  corpus's 2 firings were this exact shape). Fixed: abstain when the
  condition contains a `[cmd …]` substitution; 2 new paired FP tests
  (`analyser/bounds_checks.rs`).

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
- [x] **W303** ReDoS regexp (0 corpus) — synthetic verify. 2026-08:
  `regexp {(a+)+$} $s` fires the textbook nested-quantifier ReDoS shape
  correctly; a plain literal pattern with no nested quantifier / overlapping
  alternation does not. TP.
- [x] **W307** non-literal command name — RESOLVED (see top): proc-
  param dispatcher + multi-dispatch local heuristic, with taint guard
  to keep firing on tainted dispatch (security correctness).  Cross-
  proc object provenance (factory return-type tracking through
  interproc summaries) remains as a follow-on for the smaller
  residual.
- [x] **W308** subst without -nocommands (0 corpus) — synthetic. **Row was
  stale**: W308 is no longer this check. It was renamed/repurposed —
  `tcl-core-types/src/diag_code.rs`'s own regression test
  (`w308_documents_the_tcloo_unknown_method_check`) documents that "a
  historical mislabel described it as a subst-without--nocommands security
  warning — a check no emitter ever produced" (that hazard is covered by
  W102 / the T100 taint sink gate). W308 is now the **TclOO unknown-method**
  check. 2026-08 verified the current check: `[Foo new] baz` (no such
  method) fires "Unknown method 'baz' on class '::Foo'; did you mean
  'bar'?" correctly, with the existing dedicated test suite already
  covering it thoroughly. TP; the row above is corrected to match.
- [x] **W309** eval/uplevel with subst (0 corpus) — synthetic. 2026-08:
  `eval [subst $x]` where `$x` holds `{[exec rm -rf /]}` fires the
  double-substitution warning correctly, alongside the related W101/W102.
  TP.
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
  (1) constructor layouts were centralised in the registry's manufacturer
  descriptors. This also corrected the oracle: `createWithNamespace` has a
  distinct structural layout but is unexported on ordinary class commands in
  C Tcl 9.0.4 and 8.6, so a source call cannot be treated as successful
  construction unless reachability is separately proved. `new` is likewise
  hidden on `oo::class` itself while remaining exported on ordinary classes.
  (2) `handle_namespace_ensemble`'s `-command` extraction scanned
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
- [x] **E200** shimmer parse error (0) — synthetic. Already has a dedicated,
  `tclsh 9.0.3`-verified regression suite
  (`analyser/syntax_checks.rs::e201_brace_swallowed_command_falls_back_to_e200`
  and neighbours) covering the narrow "bracket-recovery must bail rather
  than guess" case this code exists for; not re-swept beyond confirming
  that suite is green. TP.
- [x] **H300** possible paste error (0 corpus) — synthetic. 2026-08:
  `set x 5; set x 5` (identical consecutive assignment, the classic
  copy-paste bug) fires correctly at Hint severity. TP.

## NOT YET INSPECTED — iRules (IRULE*) + Tk (TK*)

The corpus is mostly non-iRules/non-Tk, so these barely fire here. Need a
dedicated iRules corpus + the Tk stdlib for a real sweep.
- [x] **IRULE1001** (1) / **IRULE1005** (2) — only ones firing; rest 0 here.
  2026-08 sweep (`samples/irules/`, `samples/for_screenshots/*.irul`): 5 +
  3 firings. IRULE1001 ("assumes profile … on the virtual server" / "may
  not work in EVENT") — all genuine event/profile mismatches; one instance
  quotes a deliberately-misspelled event name (`HTPT_REQUEST`) from a
  sample file that exists specifically to demonstrate IRULE1002, not an
  analyser bug. IRULE1005 ("will never fire without a client …::collect
  call") — all genuine missing-`::collect` cases. TP throughout.
- [x] **IRULE1002–5007** — dedicated corpus sweep completed. The in-repo
  `.irul` corpus (`samples/irules/` + `samples/for_screenshots/`) is
  curated/small, so `scripts/dev/fetch-irules-corpus.sh` fetched nine public
  third-party iRules repositories. Eight supplied 206 sweepable files; the run
  also found 387 files carrying a `when EVENT` signature in any extension.
  Issue #1325's Unicode-boundary crash is fixed, and the sweep now completes.
  The lifecycle and flow residuals are recorded in the follow-up section above.
  The corpus is not vendored: it remains third-party code under its own
  licences in `tmp/`.

  Two corpus legs are additionally invisible to `fp-sweep`, which only walks
  recognised Tcl/iRules extensions: `f5devcentral/f5-agility-labs-irules`
  (~31 iRules embedded in `.rst` lab documents) and `f5devcentral/irules-toolbox`
  (~186 `.txt` snippets). Extracting those into sweepable files would widen the
  corpus further and is not yet done.
- [x] **TK1001/1002/1003** geometry/parent/option (0 corpus) — the Tk geometry
  W001 fix added coverage; sweep a real Tk app for TK100x FPs. 2026-08:
  TK1001 verified synthetically — mixing `pack`/`grid` on the same parent
  fires "Geometry manager conflict" correctly (matches real Tk's runtime
  error). TK1002/TK1003 not individually run this session (same emitter
  family, internal/non-configurable codes, lower audit priority than the
  40 user-toggleable ones).

---

## Cross-cutting follow-ups (known, not yet done)

- [x] **W210 `$dir` in pkgIndex.tcl** (~196 firings, the single biggest W210
  cluster) — Tcl's package machinery sets `$dir` before sourcing; needs a
  uri-gated implicit-var at the diagnostic layer (`get_diagnostics(uri=...)`).
  LSP-level, deferred. **Already resolved** — `analyser/diagnostics.rs` and
  `analyser/dataflow.rs` both special-case `pkgIndex.tcl` (path-suffix
  check, not content-based, so it doesn't false-fire on an unrelated file)
  and seed `$dir` as an implicit pre-set variable. 2026-08 verified with a
  synthetic `pkgIndex.tcl` (`package ifneeded foo 1.0 [list source [file
  join $dir foo.tcl]]`): no W210 on `$dir`. This checklist just never got
  updated when the fix landed.
- [ ] **W110 / O120 near-duplicate** — 1020+ ranges are byte-identical between
  the two. Policy call: which subsystem owns the user-facing squiggle. Still
  open — a genuine product decision (which subsystem's diagnostic a user
  sees, or whether both should keep firing), not a correctness bug; 2026-08
  sweep reconfirmed both still fire on the same real sites (7 W120 corpus
  firings this session were all clean `eq`/`ne`-worthy `==`-on-strings; no
  W110/O120 pair was inspected side-by-side for exact range overlap this
  session — the policy call itself needs a maintainer decision, not more
  sweeping).
- [ ] **W123 per-package stubs** — argparse / dict-extension (dget/dexist) /
  custom widget commands. A stub bundle would cut ~half the W123 noise.
  Still open — a content/registry-authoring task (writing stub bundles for
  common third-party packages), not an analyser defect; out of scope for
  this sweep.

## Process

- Sweep highest-volume un-inspected first (O110, then W104/W126 as likely-FP,
  then the O-series DCE family, then the long tail).
- Every behaviour change: paired TP/FP tests (mirror the FP catalog convention),
  tclsh-verified, ci-fast + the relevant suite, then test-slow stamp.
- Record confirmed-TP outcomes here too (negative results are results).
