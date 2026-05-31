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
| D1-1 | W307 object/callback suppression still mostly shape-based | ✅ FIXED (mostly) | F4 (variable-command path) + D4-F5 (cmd-sub-as-command path) both lost the `in_method` blanket; D4-F4 tightened W214 dispatcher to arity; D4-F6 removed `new`-subcommand factory heuristic; D3-P7 added `array set` literal-element evidence to override callback-key suppression.  Architectural follow-up (D3-P5, full provenance via registry) deferred. |
| D1-2 | `_CMD_SUB_RE` regex parsing of Tcl cmd-subs | ✅ ALREADY FIXED | commit `8fd84304` replaced with `parse_command_substitution` |
| D1-3 | `parse_command_substitution` uses `.split()` | ✅ ALREADY FIXED | commit `8fd84304` uses `segment_commands` |
| D1-4 | regexp/scan output vars need CFG/SSA conditional defs | ✅ FIXED (both reviewer cases) | F2 closure (same-statement dominator walk + embedded-condition F2 extension + scan no-match estimator with D4-F1 conservative bail-out) covers both reviewer examples: `regexp {x} y -> v; if {1} { puts $v }` AND `if {![regexp {x} y -> v]} { puts $v }` both fire W210.  Full CFG branch-sensitive conditional-def model is a deeper architectural refactor (not needed for the reviewer's specific cases). |
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
| D2-O126 | O126 deletes RHS with side effects (e.g. `set unused [puts side]`) | ✅ FIXED | Purity gate via `_assignment_safe_to_delete` consuming `classify_side_effects` |
| D2-O100 | O100 propagates stale constants across `[append x b]` / `[set x b]` / `[incr x]` cmd-sub writes | ✅ FIXED | `kill_sites` now includes `statement_cmd_sub_write_names` for every statement |
| D2-O109 | O109 dead-store decisions miss cmd-sub read-own-def → wrong DCE | ✅ FIXED | Same purity gate on RHS + cmd-sub-write kill_sites |
| D2-O127 | O127 load-forwarding participates in stale-fact combinations | ✅ FIXED | Resolved as side effect of O100/O109 kill_sites extension |
| **D2-O126-FU** | Extend `_assignment_safe_to_delete` to consume interprocedural purity summaries so pure user-proc RHS can also be folded to deletion. | ✅ FIXED | `optimise_elimination_passes` now builds `interproc_pure` (qnames with `summary.pure==True`) and threads it through `_word_has_observable_side_effect` / `_expr_has_observable_side_effect` / `_assignment_safe_to_delete`.  Pure user-proc cmd-subs are now allowed; impure ones (puts, file I/O, etc.) still refuse.  TclOO `method` purity (`ClassDef.method_purity`) NOT yet wired (the method-call path goes through `IRCall my <method>` which classify_side_effects treats as impure — a follow-up TODO if needed). |

All four have ONE shared root cause: command-substitution writes are not modelled as SSA kills.

---

## Doc 3: FN/TN test candidates (9 pairs, 18 snippets)

Tclsh-verified positive/negative pairs.

| ID | Pair | FN status | TN status | Closure target |
|---|---|---|---|---|
| D3-P1 | empty `dict with` + `return $missing` | ✅ FIXED | ✅ silent | Via D4-F3 (key-aware dict-with on return path) · [FP-DS-08](FP.md#fp-ds-08) |
| D3-P2 | call-site literal dict not used interproc | ✅ FIXED | ✅ silent | Two-part fix: (a) `compiler.core_analyses._collect_call_site_constants` collects per-callee literal arg values across all call sites in the IR module; `_compile_source_inner` + `analyse_ir_module` seed `param_constants={(p,0): CONST(v)}` only when EVERY caller agrees on the literal (mixed callers -> conservative fallback).  (b) SCCP barrier-widening refined to PRESERVE version-0 entries (param entry values are by construction the input-from-outside value, never re-written in the function body).  Together they let `f {}` propagate to `d#0 = CONST('')` in the callee, the dict-with key-aware logic sees no keys, and `return $missing` correctly fires W210. · [FP-DS-09](FP.md#fp-ds-09) |
| D3-P3 | `[format X] run` in method | ✅ FIXED | Via D4-F5 (in_method blanket removed) |
| D3-P4 | `[my plain] run` where plain returns string | ✅ FIXED | Lightweight method-body inspection: when `my <method>` resolves to a method in the enclosing class whose body is a simple `return <literal>` (no cmd-sub, no var interpolation), override the self-dispatch object heuristic and fire W307.  Compound bodies stay conservatively suppressed. |
| D3-P5 | `[::pkg::plain]` external returns string | 🔄 PARTIAL | D3-P5 partial closure shipped: registered ``::ns::cmd`` with EXPLICIT non-OBJECT ``return_type`` overrides the ``::``-prefix factory heuristic.  Unregistered commands like the literal reviewer example (`::pkg::plain` — not in the registry at all) still suppress W307.  Complete closure = adding ``return_type=TclType.STRING`` to specs for the small number of tcllib/Tk commands that return strings; pure data work tracked under D1-11 spec coverage. |
| D3-P6 | `[NotAClass new]` external returns string | ✅ FIXED | Via D4-F6 (`new`-subcommand heuristic on bare names removed) |
| D3-P7 | `array set state {-command notACommand}` non-cmd | ✅ FIXED | New `array set` literal-element harvester feeds SCCP CONST evidence to override the callback-key heuristic |
| D3-P8 | `dict with d { $cmd hi }` with literal `{cmd notACommand}` | ✅ FIXED | Built on D3-P2 closure: the dict-with key-value-pair harvester in `_emit_var_command_diagnostics` reads the dict_var's SCCP CONST value at v0 and registers each key->value pair in the CONSTSET map.  The W307 check then sees `cmd -> notACommand` and fires (the value isn't a known command). |
| D3-P9 | W214 unrelated dispatcher suppresses peers | ✅ FIXED | Via D4-F4 (arity-compatible dispatcher requirement) · [FP-STY-09](FP.md#fp-sty-09) |

---

## Doc 4: Post-fix review (11 findings)

| ID | Finding | Status | Closure |
|---|---|---|---|
| D4-F1 | `scan_provably_no_match` unsound (`%n`, `Inf`, `\r\f\v` in format) | ✅ FIXED | `%n` mapped to new `"always"` kind; float predicate accepts `Inf`/`Infinity`/`NaN`; format-whitespace extended to `\r\f\v`; conservative bail on backslash/$/[ in raw source text (analyser sees pre-escape source) · [FP-STY-10](FP.md#fp-sty-10) |
| D4-F2 | Variadic var-writes hard-coded in `scan`/`lassign`/`binary scan` specs | ✅ FIXED | Dynamic `arg_role_resolver` per command -- no more finite slot budget · [FP-STY-11](FP.md#fp-sty-11) |
| D4-F3 | `dict with` return-path uses blanket suppression (= D3-P1 FN) | ✅ FIXED | Mirrored key-aware logic from statement-use path to the CFGReturn arm of `_read_before_set` · [FP-DS-08](FP.md#fp-ds-08) |
| D4-F4 | W214 dispatch-protocol evidence too broad (= D3-P9 FN) | ✅ FIXED | Extended var_command_sites to record positional arg count; protocol match now requires arity-compatible dispatcher (1-arg `$cmd x` no longer suppresses 2-arg peer family) · [FP-STY-09](FP.md#fp-sty-09) |
| D4-F5 | Cmd-sub-as-command in methods still has blanket W307 (= D3-P3 FN) | ✅ FIXED | Removed blanket `in_method` suppression for cmd-sub-as-command; only `my`/`self` self-dispatch + KNOWN OBJECT return-type now suppress |
| D4-F6 | Object-factory inference from `::ns::` and `new` spelling (= D3-P5, P6) | 🔄 PARTIAL | `new`-subcommand heuristic removed; `::`-prefix heuristic kept for tcllib corpus compat but downgraded -- user procs with non-object-returning fixpoint result override.  D3-P5 (unknown external `::pkg::plain`) still suppressed pending registry coverage of tcllib factory commands. |
| D4-F7 | `${ns}::tail` source-offset scan over-fires + misses composed cmds | ✅ FIXED | Composed-name lookup runs unconditionally for namespaced ensembles -- known proc -> override `sccp_says_not_a_command`, all unknown -> set it True, mixed -> conservative |
| D4-F8 | Inline-pass proc liveness uses `_PROC_NAME_WORD_RE` Python regex | ✅ FIXED | Added whitespace-split fallback alongside the regex so proc names with non-`\w` chars (`do-work`, `+`, ...) aren't silently dropped |
| D4-F9 | iRules IRULE4004 hoistability regex-scans Tcl values | ✅ FIXED | New `_scan_namespaced_cmds_in_text` uses lexer + segmenter to find namespaced cmd-subs; recurses into args; falls back to regex only on unparseable input |
| D4-F10 | Optimiser O109/O126 overlap filter `split(None, 2)` Tcl parser bypass | ✅ FIXED | Replaced `split(None, 2)` with `segment_commands(text)` + `normalise_var_name`; also fixed O112-replacement var scanner to descend into BODY/EXPR script-role args (was missing `$b` in `if {$b} {...}`) |
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
