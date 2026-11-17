# Test scrub: taint and iRules checks

Families: `tcl-compiler` `taint`, `taint_interproc`, `irules_checks`, `analyser::irules_event_checks`, `uri_split`, `connection_scope`, `it_3/taint`, `it_4/taint_depth`. 171 failing tests (204 marked failing this morning; 33 fixed since, `uri_split` among them). Ledger: `ledger/taint.tsv`. No test was edited or deleted.

## Counts by verdict

| Verdict | Tests |
| --- | ---: |
| `FIX-FIXTURE` | 81 |
| `KEEP-CONTRACT` | 38 |
| `KEEP-ORACLE` | 15 |
| `KEEP-F5` | 13 |
| `REWRITE` | 13 |
| `UNDECIDED` | 10 |
| `FIX-EXPECTATION` | 1 |

## Causes, ranked by tests blocked

| Cause | Tests | Verdicts |
| --- | ---: | --- |
| `tk-fixture-without-tk` | 35 | FIX-FIXTURE 35 |
| `irules-checks-dialect-blind-unit` | 34 | FIX-FIXTURE 34 |
| `profileless-registry-fixture` | 11 | FIX-FIXTURE 11 |
| `taint-internal-owner-harness` | 8 | UNDECIDED 8 |
| `connection-scope-cells-missing` | 6 | REWRITE 6 |
| `callback-replay-no-warnings` | 6 | KEEP-ORACLE 6 |
| `instance-class-factory-result-dropped` | 5 | REWRITE 5 |
| `expanded-command-head` | 5 | KEEP-CONTRACT 4, KEEP-ORACLE 1 |
| `irule3103-proven-operand` | 4 | KEEP-CONTRACT 4 |
| `irule-cross-event-variable-unreported` | 3 | KEEP-F5 3 |
| `irule5001-false-warning-setup-event-def` | 3 | KEEP-F5 3 |
| `sink-in-command-substitution` | 3 | KEEP-CONTRACT 3 |
| `rule-proc-call-from-handler` | 3 | KEEP-CONTRACT 3 |
| `nested-sink-argument-substitution` | 3 | KEEP-CONTRACT 2, KEEP-ORACLE 1 |
| `irule-nested-handler-context` | 2 | KEEP-F5 2 |
| `irule-static-write-unreported` | 2 | KEEP-F5 2 |
| `irule6001-shared-name-unreported` | 2 | KEEP-F5 2 |
| `profileless-registry-input` | 2 | UNDECIDED 1, FIX-EXPECTATION 1 |
| `nested-sink-in-condition` | 2 | KEEP-CONTRACT 2 |
| `duplicate-eval-sink-warning` | 2 | KEEP-CONTRACT 1, KEEP-ORACLE 1 |
| `inline-regex-quote-colour` | 2 | KEEP-CONTRACT 1, KEEP-ORACLE 1 |
| `interproc-return-word-internal` | 2 | REWRITE 2 |
| `t102-literal-dash-prefix` | 2 | KEEP-CONTRACT 2 |
| `cross-dialect-network-sink` | 2 | KEEP-CONTRACT 2 |
| `irules-collection-source-owner` | 1 | KEEP-CONTRACT 1 |
| `irules-conditional-flow-obligations` | 1 | UNDECIDED 1 |
| `irule-unknown-event-body-analysed` | 1 | KEEP-CONTRACT 1 |
| `command-head-escape-grammar` | 1 | KEEP-CONTRACT 1 |
| `interp-alias-sink-prefix` | 1 | KEEP-CONTRACT 1 |
| `native-option-schema` | 1 | KEEP-CONTRACT 1 |
| `ensemble-subcommand-label` | 1 | KEEP-CONTRACT 1 |
| `opaque-switch-arm-taint` | 1 | KEEP-CONTRACT 1 |
| `hosted-template-sink` | 1 | KEEP-CONTRACT 1 |
| `renamed-sanitiser-not-trusted` | 1 | KEEP-CONTRACT 1 |
| `string-map-crlf-sanitiser` | 1 | KEEP-ORACLE 1 |
| `proven-path-operand` | 1 | KEEP-CONTRACT 1 |
| `cross-interp-script-word` | 1 | KEEP-CONTRACT 1 |
| `dynamic-trace-target` | 1 | KEEP-CONTRACT 1 |
| `helper-sanitiser-summary` | 1 | KEEP-ORACLE 1 |
| `oo-method-body-not-entered` | 1 | KEEP-ORACLE 1 |
| `expr-function-argument-sink` | 1 | KEEP-ORACLE 1 |
| `irules-env-source` | 1 | KEEP-F5 1 |
| `irules-fixture-uses-puts` | 1 | FIX-FIXTURE 1 |
| `helper-return-source` | 1 | KEEP-CONTRACT 1 |
| `t102-expanded-argument` | 1 | KEEP-CONTRACT 1 |
| `regsub-double-quote` | 1 | KEEP-ORACLE 1 |

## Distinct failing behaviours

Wrong answers first (the product reports something it should not, or names the wrong thing):

1. IRULE5001 fires for a debug flag that a setup event (`RULE_INIT`, `CLIENT_ACCEPTED`, a switch arm there) sets (`irule5001-false-warning-setup-event-def`, 3).
2. IRULE3102 is reported inside the body of an event the loader rejects (`irule-unknown-event-body-analysed`, 1).
3. One eval sink is reported more than once (`duplicate-eval-sink-warning`, 2).
4. A rule procedure's encoder or sanitiser result still warns at the handler's sink (`rule-proc-call-from-handler`, 3) and a helper returning `[string length]` still warns T100 (`helper-sanitiser-summary`, 1).
5. IRULE3101 warns on a proven `/` path (`proven-path-operand`, 1).

Missing answers:

6. No iRules flow check reports in its own tests, because the tests' unit has no handler (`irules-checks-dialect-blind-unit`, 38): a fixture fix, then IRULE1005-1007, 1201, 1202, 4002, 4004, 5002, 5004 are measured for real (F5 claims: BIG-IP check 071).
7. Tk widget getters are not taint sources (`tk-fixture-without-tk`, 35; `instance-class-factory-result-dropped`, 5): fixtures need `package require Tk`, and the taint consumer must read the instance method's `taint_source` through tk-oo's Instance binding.
8. Interprocedural summaries under a profile-less registry (`profileless-registry-fixture`, 11): a fixture fix to named releases.
9. Callback scripts replayed for taint give no warning (`callback-replay-no-warnings`, 6).
10. IRULE3103 over a proved operand (`irule3103-proven-operand`, 5): the value is now in `semantic_values()`.
11. Cross-event variables (IRULE4003, 3), shared names (IRULE6001, 2), static writes outside `RULE_INIT` (IRULE4001, 2), and the connection-scope records behind them (6, `REWRITE`).
12. Expanded or variable command heads reaching `eval` (`expanded-command-head`, 6); sinks inside command substitutions (`sink-in-command-substitution`, `nested-sink-argument-substitution`, `nested-sink-in-condition`, 9).
13. Smaller groups, one or two tests each: T102 literal dash prefix, ensemble labels, interp alias sink prefix, method bodies not entered, `env` in TMM (check 073), the rest in the cause table.

`UNDECIDED` (10): nine harness panics while locating an internal owner (each needs a consumer path written before it can be `REWRITE` or `RETIRE`), and one internal obligation inventory in `irules_checks`. The profile-less sink-owner test waits on whether production builds a profile-less registry.

## Overlap groups touching these families

All members pass; they are for the second pass. Exact copies (8 groups, 16 tests): `taint::option_sinks::t102_suppression` against `taint::sources::source_colour_augmentation` (four colours), `taint::colours::shell_atom` against `t102_suppression` (two), `crlf_free` against `url_encoded`, and the `${uri}/suffix` interpolation pair. Overlap (16 groups, 33 tests) adds `irules_checks` payload families, T104 message and socket host, `eval_sink` against `taint_warning_message`, `puts` content against `puts_position_filter`, and the `HTTP::uri` getter pair. Listed in `data/clusters_exact.tsv` and `data/clusters_overlap.tsv`.
