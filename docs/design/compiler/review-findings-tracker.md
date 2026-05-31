# Review-findings tracker

Every finding from the four review docs uploaded 2026-05-31, with current
status and verification.  Status legend:

* ⬜ TODO — not yet addressed
* 🔄 IN-PROGRESS — partial fix landed, more work needed
* ✅ FIXED — closed with regression test pinning the verdict
* ✅ ALREADY FIXED — earlier work closed this before the review was written
* ❌ NOT-A-BUG — review claim verified false against tclsh
* 📋 DEFERRED — tracked, larger work, intentionally out of scope this session

Verification convention: each finding is verified against the snippet
in the review doc using `get_diagnostics()` and (where applicable) real
`tclsh9.0`.  Closed items list the commit + regression test.

---

## Doc 1: PR498 special-casing review (12 findings)

Verified against current HEAD in `tests/test_fp_*` + `tests/test_ground_truth_tn_fn.py`.

| ID | Finding | Status | Closure |
|---|---|---|---|
| D1-1 | W307 object/callback suppression still mostly shape-based | ✅ FIXED (mostly) | F4 (variable-command path) + D4-F5 (cmd-sub-as-command path) both lost the `in_method` blanket; D4-F4 tightened W214 dispatcher to arity; D4-F6 removed `new`-subcommand factory heuristic; D3-P7 added `array set` literal-element evidence to override callback-key suppression.  Architectural follow-up (D3-P5, full provenance via registry) deferred. · [FP-OBJ-12](FP.md#fp-obj-12) · [FP-OBJ-15](FP.md#fp-obj-15) · [FP-OBJ-17](FP.md#fp-obj-17) |
| D1-2 | `_CMD_SUB_RE` regex parsing of Tcl cmd-subs | ✅ ALREADY FIXED | commit `8fd84304` replaced with `parse_command_substitution` |
| D1-3 | `parse_command_substitution` uses `.split()` | ✅ ALREADY FIXED | commit `8fd84304` uses `segment_commands` |
| D1-4 | regexp/scan output vars need CFG/SSA conditional defs | ✅ FIXED (both reviewer cases) | F2 closure (same-statement dominator walk + embedded-condition F2 extension + scan no-match estimator with D4-F1 conservative bail-out) covers both reviewer examples: `regexp {x} y -> v; if {1} { puts $v }` AND `if {![regexp {x} y -> v]} { puts $v }` both fire W210.  Full CFG branch-sensitive conditional-def model is a deeper architectural refactor (not needed for the reviewer's specific cases). · [FP-RBS-12](FP.md#fp-rbs-12) |
| D1-5 | Python `re` regexp estimator not conservative | ✅ FIXED | commit `569eaf84` (F5 closure: option-aware estimator, conservative bail) |
| D1-6 | Proc arg traits need caller-frame vs callee-local | ✅ FIXED | DYNAMIC_NAME_LOCAL trait + per-handler integration verified |
| D1-7 | W214 dispatch-protocol evidence too broad | ✅ FIXED | D4-F4 closure tightens the heuristic to require arity-compatible dispatcher; unrelated 1-arg `$cmd x` no longer suppresses 2-arg peer family. · [FP-STY-09](FP.md#fp-sty-09) |
| D1-8 | W304 uses lexical last-set scan | ✅ ALREADY FIXED | Earlier follow-up wave |
| D1-9 | Optimiser regex `\$(\w+)` for var scan | ✅ ALREADY FIXED | Replaced with `VarReferenceScanner` |
| D1-10 | Place / var-ref command-walking duplicated | ✅ MOSTLY FIXED | The shared services the reviewer called for are already implemented: `compiler.parsing.command_segmenter.segment_commands`, `compiler.registry.runtime.arg_indices_for_role[s]`, `compiler.var_refs.VarReferenceScanner`.  Heavily used in this session's closures (F11/F10/F9/F8/F7).  Remaining work is mechanical migration of legacy regex-based consumers; D4-F8/F9/F10/F11 closures completed that for the worst offenders.  Per-pass duplication that remains (e.g. SSA's pre-uses extraction in `_uses` vs `var_refs.scan_script`) is justified by different consumer needs (SSA needs versioned defs/uses, VarReferenceScanner needs name-set). |
| D1-11 | Registry missing facts (conditional outputs, callback roles, factory returns, …) | ✅ MOSTLY FIXED | Each fact the reviewer listed maps to existing infrastructure consumed in this session: conditional outputs (D4-F2 variadic + D4-F1 no-match), callback roles (ArgRole enums incl. DYNAMIC_NAME_LOCAL), object factory return (D3-P5 partial -- `CommandSpec.return_type`/`SubCommand.return_type` consulted), structural body scope (`BodyKind`), var-name scope (VAR_WRITE / VAR_READ / DYNAMIC_NAME_LOCAL trio), purity (`classify_side_effects` + interproc summaries -- consumed by D2-O126 / D2-O126-FU).  Remaining is per-command spec coverage (tcllib `struct::*`, `pt::*` factories; specific callback options) -- data work, not architectural. |
| D1-12 | Registry could supply role-guided descent plans | ✅ ALREADY EXISTS | The descent-plan API the reviewer suggested is already public: `arg_indices_for_role(cmd, args, role)`, `arg_indices_for_roles(cmd, args, roles)` (bundled lookup), and `resolve_arg_role_map(cmd, args)` (full plan).  Memoised via `_resolve_arg_roles`.  Consumers should call these instead of re-scanning -- the recent inline-pass / iRules / optimiser cleanups all use them.  The phrasing in the doc was forward-looking; the cited code is already what's recommended. |

---

## Doc 2: Optimisation soundness sweep against C Tcl 9 (4 unsoundness findings)

Verified by running `optimise_source()` and comparing tclsh stdout/stderr.

| ID | Finding | Status | Closure |
|---|---|---|---|
| D2-O126 | O126 deletes RHS with side effects (e.g. `set unused [puts side]`) | ✅ FIXED | Purity gate via `_assignment_safe_to_delete` consuming `classify_side_effects` · [FP-OPT-05](FP.md#fp-opt-05) |
| D2-O100 | O100 propagates stale constants across `[append x b]` / `[set x b]` / `[incr x]` cmd-sub writes | ✅ FIXED | `kill_sites` now includes `statement_cmd_sub_write_names` for every statement · [FP-OPT-06](FP.md#fp-opt-06) |
| D2-O109 | O109 dead-store decisions miss cmd-sub read-own-def → wrong DCE | ✅ FIXED | Same purity gate on RHS + cmd-sub-write kill_sites · [FP-OPT-06](FP.md#fp-opt-06) |
| D2-O127 | O127 load-forwarding participates in stale-fact combinations | ✅ FIXED | Resolved as side effect of O100/O109 kill_sites extension · [FP-OPT-06](FP.md#fp-opt-06) |
| **D2-O126-FU** | Extend `_assignment_safe_to_delete` to consume interprocedural purity summaries so pure user-proc RHS can also be folded to deletion. | ✅ FIXED | `optimise_elimination_passes` now builds `interproc_pure` (qnames with `summary.pure==True`) and threads it through `_word_has_observable_side_effect` / `_expr_has_observable_side_effect` / `_assignment_safe_to_delete`.  Pure user-proc cmd-subs are now allowed; impure ones (puts, file I/O, etc.) still refuse.  TclOO `method` purity (`ClassDef.method_purity`) NOT yet wired (the method-call path goes through `IRCall my <method>` which classify_side_effects treats as impure — a follow-up TODO if needed). · [FP-OPT-07](FP.md#fp-opt-07) |

All four have ONE shared root cause: command-substitution writes are not modelled as SSA kills.

---

## Doc 3: FN/TN test candidates (9 pairs, 18 snippets)

Tclsh-verified positive/negative pairs.

| ID | Pair | FN status | TN status | Closure target |
|---|---|---|---|---|
| D3-P1 | empty `dict with` + `return $missing` | ✅ FIXED | ✅ silent | Via D4-F3 (key-aware dict-with on return path) · [FP-DS-08](FP.md#fp-ds-08) |
| D3-P2 | call-site literal dict not used interproc | ✅ FIXED | ✅ silent | Two-part fix: (a) `compiler.core_analyses._collect_call_site_constants` collects per-callee literal arg values across all call sites in the IR module; `_compile_source_inner` + `analyse_ir_module` seed `param_constants={(p,0): CONST(v)}` only when EVERY caller agrees on the literal (mixed callers -> conservative fallback).  (b) SCCP barrier-widening refined to PRESERVE version-0 entries (param entry values are by construction the input-from-outside value, never re-written in the function body).  Together they let `f {}` propagate to `d#0 = CONST('')` in the callee, the dict-with key-aware logic sees no keys, and `return $missing` correctly fires W210. · [FP-DS-09](FP.md#fp-ds-09) |
| D3-P3 | `[format X] run` in method | ✅ FIXED | Via D4-F5 (in_method blanket removed) · [FP-OBJ-12](FP.md#fp-obj-12) |
| D3-P4 | `[my plain] run` where plain returns string | ✅ FIXED | Lightweight method-body inspection: when `my <method>` resolves to a method in the enclosing class whose body is a simple `return <literal>` (no cmd-sub, no var interpolation), override the self-dispatch object heuristic and fire W307.  Compound bodies stay conservatively suppressed. · [FP-OBJ-13](FP.md#fp-obj-13) |
| D3-P5 | `[::pkg::plain]` external returns string | 🔄 PARTIAL | D3-P5 partial closure shipped: registered ``::ns::cmd`` with EXPLICIT non-OBJECT ``return_type`` overrides the ``::``-prefix factory heuristic.  Unregistered commands like the literal reviewer example (`::pkg::plain` — not in the registry at all) still suppress W307.  Complete closure = adding ``return_type=TclType.STRING`` to specs for the small number of tcllib/Tk commands that return strings; pure data work tracked under D1-11 spec coverage. · [FP-OBJ-14](FP.md#fp-obj-14) |
| D3-P6 | `[NotAClass new]` external returns string | ✅ FIXED | Via D4-F6 (`new`-subcommand heuristic on bare names removed) · [FP-OBJ-15](FP.md#fp-obj-15) |
| D3-P7 | `array set state {-command notACommand}` non-cmd | ✅ FIXED | New `array set` literal-element harvester feeds SCCP CONST evidence to override the callback-key heuristic · [FP-OBJ-17](FP.md#fp-obj-17) |
| D3-P8 | `dict with d { $cmd hi }` with literal `{cmd notACommand}` | ✅ FIXED | Built on D3-P2 closure: the dict-with key-value-pair harvester in `_emit_var_command_diagnostics` reads the dict_var's SCCP CONST value at v0 and registers each key->value pair in the CONSTSET map.  The W307 check then sees `cmd -> notACommand` and fires (the value isn't a known command). · [FP-OBJ-18](FP.md#fp-obj-18) |
| D3-P9 | W214 unrelated dispatcher suppresses peers | ✅ FIXED | Via D4-F4 (arity-compatible dispatcher requirement) · [FP-STY-09](FP.md#fp-sty-09) |

---

## Doc 4: Post-fix review (11 findings)

| ID | Finding | Status | Closure |
|---|---|---|---|
| D4-F1 | `scan_provably_no_match` unsound (`%n`, `Inf`, `\r\f\v` in format) | ✅ FIXED | `%n` mapped to new `"always"` kind; float predicate accepts `Inf`/`Infinity`/`NaN`; format-whitespace extended to `\r\f\v`; conservative bail on backslash/$/[ in raw source text (analyser sees pre-escape source) · [FP-STY-10](FP.md#fp-sty-10) |
| D4-F2 | Variadic var-writes hard-coded in `scan`/`lassign`/`binary scan` specs | ✅ FIXED | Dynamic `arg_role_resolver` per command -- no more finite slot budget · [FP-STY-11](FP.md#fp-sty-11) |
| D4-F3 | `dict with` return-path uses blanket suppression (= D3-P1 FN) | ✅ FIXED | Mirrored key-aware logic from statement-use path to the CFGReturn arm of `_read_before_set` · [FP-DS-08](FP.md#fp-ds-08) |
| D4-F4 | W214 dispatch-protocol evidence too broad (= D3-P9 FN) | ✅ FIXED | Extended var_command_sites to record positional arg count; protocol match now requires arity-compatible dispatcher (1-arg `$cmd x` no longer suppresses 2-arg peer family) · [FP-STY-09](FP.md#fp-sty-09) |
| D4-F5 | Cmd-sub-as-command in methods still has blanket W307 (= D3-P3 FN) | ✅ FIXED | Removed blanket `in_method` suppression for cmd-sub-as-command; only `my`/`self` self-dispatch + KNOWN OBJECT return-type now suppress · [FP-OBJ-12](FP.md#fp-obj-12) |
| D4-F6 | Object-factory inference from `::ns::` and `new` spelling (= D3-P5, P6) | 🔄 PARTIAL | `new`-subcommand heuristic removed; `::`-prefix heuristic kept for tcllib corpus compat but downgraded -- user procs with non-object-returning fixpoint result override.  D3-P5 (unknown external `::pkg::plain`) still suppressed pending registry coverage of tcllib factory commands. · [FP-OBJ-14](FP.md#fp-obj-14) · [FP-OBJ-15](FP.md#fp-obj-15) |
| D4-F7 | `${ns}::tail` source-offset scan over-fires + misses composed cmds | ✅ FIXED | Composed-name lookup runs unconditionally for namespaced ensembles -- known proc -> override `sccp_says_not_a_command`, all unknown -> set it True, mixed -> conservative · [FP-OBJ-16](FP.md#fp-obj-16) |
| D4-F8 | Inline-pass proc liveness uses `_PROC_NAME_WORD_RE` Python regex | ✅ FIXED | Added whitespace-split fallback alongside the regex so proc names with non-`\w` chars (`do-work`, `+`, ...) aren't silently dropped |
| D4-F9 | iRules IRULE4004 hoistability regex-scans Tcl values | ✅ FIXED | New `_scan_namespaced_cmds_in_text` uses lexer + segmenter to find namespaced cmd-subs; recurses into args; falls back to regex only on unparseable input |
| D4-F10 | Optimiser O109/O126 overlap filter `split(None, 2)` Tcl parser bypass | ✅ FIXED | Replaced `split(None, 2)` with `segment_commands(text)` + `normalise_var_name`; also fixed O112-replacement var scanner to descend into BODY/EXPR script-role args (was missing `$b` in `if {$b} {...}`) · [FP-OPT-08](FP.md#fp-opt-08) |
| D4-F11 | `is_pure_var_ref()` Python regex over Tcl variable syntax | ✅ FIXED | Hand-rolled Tcl-correct parser `_scan_pure_var_ref`; handles backslash-escaped close-paren in array index (reviewer's `$a(x\)y)` case) · [FP-NAB-12](FP.md#fp-nab-12) |

---

## Working order

Critical correctness first (changes program behaviour):

1. **D4-F1** scan unsoundness — small, isolated, in-progress
2. **D2-O126** side-effectful RHS deletion — purity gate on the RHS
3. **D2-O100/O109/O127** cmd-sub writes as SSA kills — single shared fix

Precision improvements (changes diagnostic verdicts, no runtime behaviour):

4. **D4-F3 / D3-P1** dict-with return-path key-aware
5. **D4-F2** variadic var-writes for scan/lassign/binary scan
6. **D4-F7** namespaced-ensemble using SCCP
7. **D4-F6 / D3-P5, P6** object-factory provenance
8. **D4-F5 / D3-P3, P4** cmd-sub-as-cmd in methods
9. **D3-P7** array callback key value evidence
10. **D4-F4 / D3-P9** W214 dispatcher evidence
11. **D3-P2, P8** interproc literal-dict propagation

Code-quality cleanups (regex → parser):

12. **D4-F8** inline-pass liveness
13. **D4-F9** iRules hoistability
14. **D4-F10** optimiser overlap filter
15. **D4-F11** `is_pure_var_ref()`

Architectural deferreds (D1-10, D1-11, D1-12) tracked separately.

---

## Doc 5: Compiler optimisation, shimmer & taint follow-up review (8 findings, 2026-05-31)

Re-review of `bitwisecook/tcl-lsp#499` at `b8654117`.  Previous O100/O109/O126/O127
cmd-sub / RHS-deletion issues confirmed clean.  New high-confidence issues:

All 8 verified against `/usr/local/bin/tclsh9.0` (Tcl 9.0.3).

| ID | Severity | Finding | Status | Closure |
|---|---|---|---|---|
| D5-O110 | HIGH | O110 InstCombine drops Tcl numeric coercion/error semantics (`$x + 0`, `$x * 1`, `$x * 0`, `$x / 1`, `$x << 0`, `$x & 0`, `$x % 1` removed without proving operand is numeric).  tclsh: `expr {"abc" + 0}` ERRORS but `expr {"abc"}` returns `"abc"` — the rewrite changes a tclsh error into successful string output. | ✅ FIXED | `_simplify_expr_node` / `_simplify_to_fixpoint` / `_instcombine_expr` now thread `ssa_uses` + `types` and consult `_is_provably_numeric_expr_node` before every drop-operand identity/annihilator rewrite (`+0`, `-0`, `*0`, `*1`, `/1`, `<<0`, `>>0`, `&0`, `|0`, `^0`, `%1`, `**0`, `**1`, `x^x`, `x-x`, `+x`, `-(-x)`, `~~x`).  Provably-numeric = literal int/float/boolean, numeric-text ExprString, or ExprVar with SSA type INT/DOUBLE/NUMERIC/BOOLEAN.  Otherwise the rewrite is skipped (sound > optimal). · [FP-OPT-09](FP.md#fp-opt-09) |
| D5-O114 | HIGH | O114 rewrites `set x [expr {$x + 1}]` to `incr x` without integer proof.  tclsh: `expr {$x + 1}` on `1.5` = `2.5`; `incr` on `1.5` errors `expected integer but got "1.5"`. | ✅ FIXED | `optimise_incr_idioms` now takes `analysis` and gates the rewrite on `analysis.types[(var, ver)].tcl_type is TclType.INT` (KNOWN INT, not DOUBLE/NUMERIC/BOOLEAN/OBJECT/unknown).  `_try_incr_idiom` gained a `var_is_int=False` kwarg that bails fast.  Loop-counter pattern (for-init INT) still fires; arbitrary param `x` does not. · [FP-OPT-10](FP.md#fp-opt-10) |
| D5-O120 | HIGH | O120 rewrites numeric `==`/`!=` to string `eq`/`ne` even when both operands could be numeric-looking.  tclsh: `1.0 == "1"` is `1` (numeric); `1.0 eq "1"` is `0` (string) — the rewrite flips the result. | ✅ FIXED | `_rewrite_eq_ne_string_compare_node` now requires AT LEAST ONE operand provably non-numeric (literal text fails `_is_numeric_string_value` OR SCCP CONST value is non-numeric).  KNOWN STRING type alone rejected -- a STRING-typed value can hold numeric-looking text.  At-least-one rule is sound per Tcl `==` semantics (string path iff at least one operand can't parse as a number) and preserves the dominant `$a == "hello"` idiom · [FP-OPT-11](FP.md#fp-opt-11) |
| D5-SH-EXPR | HIGH | `_find_expr_shimmers` only analyses `IRAssignExpr`, misses standalone `expr`, `if {…}`, `while {…}`, `for {…}` expr contexts.  All four are real expr lex-promotion sites in tclsh. | ⬜ TODO | — |
| D5-SH-EQ | MEDIUM | Shimmer treats `==`/`!=` as universally numeric.  tclsh: `$s == "hello"` with `s=hello` short-circuits to STRING comparison (both operands non-numeric); no shimmer happens.  Currently fires S100 falsely. | ✅ FIXED | `BinOp.EQ`/`BinOp.NE` partitioned out of `_NUMERIC_OPS` into a new `_CONDITIONAL_NUMERIC_OPS`.  `_collect_expr_shimmers` now fires shimmer on `==`/`!=` operands only when at least one operand is provably numeric (ExprLiteral, numeric-text ExprString, KNOWN INT/DOUBLE/NUMERIC/BOOLEAN ExprVar, or SCCP CONST that parses as a number).  Both-non-numeric stays silent (string-compare short-circuit). · [FP-SH-08](FP.md#fp-sh-08) |
| D5-T100 | HIGH | T100/T105 suppression on `LIST_CANONICAL` is unsound: `eval [list $raw]` where `$raw` is tainted runs `$raw` as a command word (proved with `proc marker args {puts EXECUTED}; eval [list marker]` → `EXECUTED`).  `LIST_CANONICAL` only proves word boundaries, NOT command-word trustedness. | ✅ FIXED | `_should_suppress_t100` / `_should_suppress_sink_warning("T105", ...)` no longer consult LIST_CANONICAL.  Suppression is granted only when the eval/uplevel/interp-eval arg is a literal ``[list <known-cmd> ...]`` cmd-sub (head in `REGISTRY.specs_by_name`) AND the tainted var sits at list-index >= 1.  Propagated lists (`set lst [list $raw]; eval $lst`) and head-tainted lists (`[list $raw]`) now fire correctly.  `taint_sink_safe_colour=LIST_CANONICAL` removed from `dialects/tcl/eval.py` and `dialects/tcl/uplevel.py`. · [FP-TNT-03](FP.md#fp-tnt-03) |
| D5-T102 | HIGH | T102 suppressed when `--` appears anywhere ≥ `scan_start`, but a `--` AFTER a tainted option candidate cannot protect that candidate.  tclsh: `regexp -- -nocase ABC` treats `-nocase` as pattern (rc=0 match=0); `regexp -nocase -- ABC abc` treats it as option (rc=0 match=1). | ✅ FIXED | `_classify_sink` no longer uses `_has_option_terminator` to globally suppress the T102 sink — it always emits T102 when the command has an option-terminator profile and delegates per-var protection to `_option_scan_region`, which already stops at the first `--` (positions before the `--` remain in the scan region and fire correctly).  `_has_option_terminator` replaced with `_option_terminator_index` for clarity. · [FP-TNT-04](FP.md#fp-tnt-04) |
| D5-T104 | MEDIUM | T104 ignores `taint_network_sink_args` argument positions in the registry — fires on any tainted var in the statement (e.g. `http::geturl URL -headers $hdr` fires for `$hdr`, but the URL is positional[0], `$hdr` is an option value). | ⬜ TODO | — |

---

## Stage-2 follow-ons (deferred from earlier waves)

| ID | Finding | Status | Notes |
|---|---|---|---|
| SF-1 | Registry data coverage: add `return_type=TclType.STRING` / `OBJECT` to specific tcllib/Tk commands so D3-P5 / D4-F6 partial closures catch unregistered external factories (`::pkg::plain` style). | ⬜ TODO | Pure data work; no architecture left.  Would chip away at the residual W307 corpus count. |
| SF-2 | Wire `ClassDef.method_purity` into D2-O126-FU's `interproc_pure` set so pure-method RHS (`set unused [my pure_method ...]`) can be safely folded. | ⬜ TODO | Small, scoped.  `IRCall my <method>` currently goes through `classify_side_effects` which conservatively treats methods as impure. |

---

## Doc 5 + stage-2 working order

Critical correctness (changes program behaviour or hides security issues):

1. **D5-O110** numeric-coercion preserving guards on identity/annihilator rewrites
2. **D5-O114** integer-domain proof gate on set/expr→incr
3. **D5-O120** numeric-vs-string `==/!=` rewrite gate (value-domain aware)
4. **D5-T100/T105** drop LIST_CANONICAL suppression for eval/uplevel/interp eval; require trusted command-word evidence
5. **D5-T102** option-region-aware `--` suppression (only suppress vars whose argument index is past a real terminator)

Precision (changes diagnostics, no runtime behaviour):

6. **D5-SH-EXPR** expr-shimmer collection from `IRExprEval` + branch/loop terminator exprs (not just `IRAssignExpr`)
7. **D5-SH-EQ** value-sensitive `==/!=` shimmer (numeric only when operands provably numeric-compatible)
8. **D5-T104** thread `taint_network_sink_args` through `TaintSinkInfo`; position-filter T104

Stage-2 follow-ons:

9. **SF-2** TclOO method purity for O126
10. **SF-1** Registry return-type coverage for tcllib/Tk factories
