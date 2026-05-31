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
| D1-1 | W307 object/callback suppression still mostly shape-based | 🔄 PARTIAL | F4 closure removed `in_method` blanket for `$var cmd`; cmd-sub-as-command path still in-method-suppressed (= D4-F5). |
| D1-2 | `_CMD_SUB_RE` regex parsing of Tcl cmd-subs | ✅ ALREADY FIXED | commit `8fd84304` replaced with `parse_command_substitution` |
| D1-3 | `parse_command_substitution` uses `.split()` | ✅ ALREADY FIXED | commit `8fd84304` uses `segment_commands` |
| D1-4 | regexp/scan output vars need CFG/SSA conditional defs | 🔄 PARTIAL | F2 closure handles same-statement and embedded-condition cases; full CFG conditional-def model deferred (D4-F3 covers the dict-with return path). |
| D1-5 | Python `re` regexp estimator not conservative | ✅ FIXED | commit `569eaf84` (F5 closure: option-aware estimator, conservative bail) |
| D1-6 | Proc arg traits need caller-frame vs callee-local | ✅ FIXED | DYNAMIC_NAME_LOCAL trait + per-handler integration verified |
| D1-7 | W214 dispatch-protocol evidence too broad | 🔄 PARTIAL | Namespace-scoped now; still over-suppresses peers when an unrelated `$cmd` exists in the same namespace (= D4-F4 / D3-P9). |
| D1-8 | W304 uses lexical last-set scan | ✅ ALREADY FIXED | Earlier follow-up wave |
| D1-9 | Optimiser regex `\$(\w+)` for var scan | ✅ ALREADY FIXED | Replaced with `VarReferenceScanner` |
| D1-10 | Place / var-ref command-walking duplicated | 📋 DEFERRED | Architectural refactor — shared service required |
| D1-11 | Registry missing facts (conditional outputs, callback roles, factory returns, …) | 📋 DEFERRED | Multi-finding follow-up — see D3 pairs for concrete examples |
| D1-12 | Registry could supply role-guided descent plans | 📋 DEFERRED | Perf optimisation |

---

## Doc 2: Optimisation soundness sweep against C Tcl 9 (4 unsoundness findings)

Verified by running `optimise_source()` and comparing tclsh stdout/stderr.

| ID | Finding | Status | Closure |
|---|---|---|---|
| D2-O126 | O126 deletes RHS with side effects (e.g. `set unused [puts side]`) | ✅ FIXED | Purity gate via `_assignment_safe_to_delete` consuming `classify_side_effects` |
| D2-O100 | O100 propagates stale constants across `[append x b]` / `[set x b]` / `[incr x]` cmd-sub writes | ✅ FIXED | `kill_sites` now includes `statement_cmd_sub_write_names` for every statement |
| D2-O109 | O109 dead-store decisions miss cmd-sub read-own-def → wrong DCE | ✅ FIXED | Same purity gate on RHS + cmd-sub-write kill_sites |
| D2-O127 | O127 load-forwarding participates in stale-fact combinations | ✅ FIXED | Resolved as side effect of O100/O109 kill_sites extension |
| **D2-O126-FU** | **FOLLOW-UP: O126 / O109 purity gate is currently registry-trait only -- user-defined procs and TclOO methods are conservatively treated as impure even when the analyser can prove otherwise.** Extend `_assignment_safe_to_delete` to consume interprocedural purity summaries (`analysis.interproc.purity[qname]`) and TclOO method purity (`ClassDef.method_purity`).  We already have side-effect classification for registry commands; the interproc layer already computes purity for user procs.  Connecting the two would let `set unused [my pureMethod]` and `set unused [pureUserProc 1]` correctly fold to deletion.  Sound by construction: missing summary → fall back to current conservative refusal. | ⬜ TODO | New finding tracked at user's request |

All four have ONE shared root cause: command-substitution writes are not modelled as SSA kills.

---

## Doc 3: FN/TN test candidates (9 pairs, 18 snippets)

Tclsh-verified positive/negative pairs.

| ID | Pair | FN status | TN status | Closure target |
|---|---|---|---|---|
| D3-P1 | empty `dict with` + `return $missing` | ⬜ TODO (FN) | ✅ already silent | D4-F3: extend key-aware logic to return path |
| D3-P2 | call-site literal dict not used interproc | ⬜ TODO (FN) | ✅ already silent | Interproc dict propagation |
| D3-P3 | `[format X] run` in method | ⬜ TODO (FN) | ✅ already silent | D4-F5: remove in-method cmd-sub-as-cmd blanket |
| D3-P4 | `[my plain] run` where plain returns string | ⬜ TODO (FN) | ✅ already silent | Use method return-type facts |
| D3-P5 | `[::pkg::plain]` external returns string | ⬜ TODO (FN) | ✅ already silent | D4-F6: don't infer object from `::` |
| D3-P6 | `[NotAClass new]` external returns string | ⬜ TODO (FN) | ✅ already silent | D4-F6: don't infer object from `new` |
| D3-P7 | `array set state {-command notACommand}` non-cmd | ⬜ TODO (FN) | ✅ already silent | Harvest literal element value |
| D3-P8 | `dict with d { $cmd hi }` with literal `{cmd notACommand}` | ⬜ TODO (FN) | ✅ already silent | Interproc literal dict propagation |
| D3-P9 | W214 unrelated dispatcher suppresses peers | ⬜ TODO (FN) | ✅ already silent | D4-F4: tie dispatch evidence to actual call graph |

---

## Doc 4: Post-fix review (11 findings)

| ID | Finding | Status | Closure |
|---|---|---|---|
| D4-F1 | `scan_provably_no_match` unsound (`%n`, `Inf`, `\r\f\v` in format) | 🔄 IN-PROGRESS | Fix in `compiler/scan_format.py` started |
| D4-F2 | Variadic var-writes hard-coded in `scan`/`lassign`/`binary scan` specs | ⬜ TODO | Need dynamic resolver |
| D4-F3 | `dict with` return-path uses blanket suppression (= D3-P1 FN) | ⬜ TODO | Apply key-aware logic to CFGReturn path |
| D4-F4 | W214 dispatch-protocol evidence too broad (= D3-P9 FN) | ⬜ TODO | Real dispatch-family evidence |
| D4-F5 | Cmd-sub-as-command in methods still has blanket W307 (= D3-P3 FN) | ⬜ TODO | Use return-type facts; resolve `my`/`self` |
| D4-F6 | Object-factory inference from `::ns::` and `new` spelling (= D3-P5, P6) | ⬜ TODO | Use registry/class/proc evidence only |
| D4-F7 | `${ns}::tail` source-offset scan over-fires + misses composed cmds | ⬜ TODO | Use SCCP + known-procs lookup |
| D4-F8 | Inline-pass proc liveness uses `_PROC_NAME_WORD_RE` Python regex | ⬜ TODO | Use compiler facts (tokens, SCCP) |
| D4-F9 | iRules IRULE4004 hoistability regex-scans Tcl values | ⬜ TODO | Recurse via segmenter |
| D4-F10 | Optimiser O109/O126 overlap filter `split(None, 2)` Tcl parser bypass | ⬜ TODO | Carry defining var name on Optimisation record |
| D4-F11 | `is_pure_var_ref()` Python regex over Tcl variable syntax | ⬜ TODO | Lexer-backed exact-word check |

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
