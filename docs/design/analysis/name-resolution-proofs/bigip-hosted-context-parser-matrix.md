# naming.bigip.hosted-context-parser-matrix

Kind: `native-observation`

## Problem statement

TMM, CLI script, iApp implementation, iCall and standalone appliance Tcl can share syntax while exposing different command surfaces. A broad context substitution would hide an unreached or unavailable door.

## Question

Which of the exact92 parser cases and environment observations agree across the five recorded contexts?

## Conclusion

The report retains grammar/newline/comment/numeral rows and separately recorded command environments. iApp implementation/iCall agree for these92cases, but neither supplies an APL/presentation answer. The initial matches equality/non-substring pair is insufficient to prove equality; the separate nine-row expression discriminator record establishes whole-string glob behaviour.

## Scope

Five recorded appliance contexts and exact92-case source; this question excludes an untested APL context and general context equivalence. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported.

The report retains grammar/newline/comment/numeral rows and separately recorded command environments. iApp implementation/iCall agree for these92cases, but neither supplies an APL/presentation answer. The initial matches equality/non-substring pair is insufficient to prove equality; the separate nine-row expression discriminator record establishes whole-string glob behaviour.

The measured report rows and their limits are:


Run `r2286l` evaluated the canonical 34 cases from `suites/10-context-parity.cases` plus 58 separately identified additions from [deep-context.cases](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/deep-context.cases`): 92 unique case IDs. The generator materialised each source from hex so LF, CRLF, physical backslash-newline, braces, comment placement and literal escapes were byte-controlled. In every wrapper the hex was converted to a bytearray and evaluated dynamically; these rows measure runtime parsing and remain separate from the literal rule-load results above. TMM cases were split across ten iRules; every chunk emitted exactly once on each actual TMM unit (`0:0` through `0:3`). tmsh, iApp, iCall, `tclsh8.4` and `tclsh8.5` each emitted all 92 results plus one environment row.

| Surface | Measured environment | 92-case relationship |
| --- | --- | --- |
| TMM iRule `RULE_INIT` | Tcl 8.4.6; `exec` absent; Tcl package only; fabricated BIG-IP `tcl_platform`; `tmsh::version` absent at runtime | Same tested F5 grammar as the other three F5 contexts; distinct command/environment surface |
| tmsh `cli script` | Tcl 8.4.6; reported `tcl_patchLevel=UNSET`; `exec` works; Tcl package only; empty `tcl_platform`; tmsh 21.1.0.1 | Same tested F5 grammar; distinct environment |
| iApp implementation | Tcl 8.4.6; `exec` works; 95 commands; populated 32-bit Linux-like platform; large ambient package set; tmsh 21.1.0.1 | Byte-identical to iCall for every case and reported environment |
| iCall script | Tcl 8.4.6; same reported command, package and platform surface as iApp; executed by a triggered handler in `scriptd` | Byte-identical to iApp for every case and reported environment |
| appliance `tclsh8.4` | Tcl 8.4.13; 85 commands | Control only; multiple grammar and surface differences |
| appliance `tclsh8.5` | Tcl 8.5.13; 92 commands | Control only; multiple grammar, index and surface differences |

Selected discriminators:

| Exact source or case | Four F5-hosted contexts | Host controls |
| --- | --- | --- |
| `set m before; if {1}{#do something}; set m` | rc 0, `before`; `#` comments through the end of the braced body | 8.4/8.5 rc 1, `extra characters after close-brace` |
| Same body with LF before `set m after` | rc 0, `after` | 8.4/8.5 same close-brace error |
| `if {1}` then body on next LF line | rc 0, `after` | 8.4/8.5 `wrong # args: no script following "1" argument` |
| Same with CRLF | rc 0, `after`; identical to LF | 8.4/8.5 same wrong-args result |
| Physical backslash-LF between condition and body | rc 0, `after` | rc 0, `after` |
| Backslash-space-LF | rc 0, empty result | 8.4/8.5 rc 1, close-brace error |
| `expr {"abcd" matches "abcd"}` | `1` | operator rejected |
| `expr {"abcd" matches "bc"}` | `0`; bare `matches` is exact equality for this discriminating pair | operator rejected |
| `expr {1 or 0 matches 0}` | `1`; `matches` binds more tightly than `or` in this probe | operator rejected |
| `expr {not "abc" starts_with "a"}` | rc 1, `can't use non-numeric string as operand of "!"`; `not` binds to the string before `starts_with` | operator syntax rejected |
| `lindex {a b} end+1` | rc 1, `bad index "end+1": must be integer or end?-integer?` | 8.4 same error; 8.5 rc 0, empty |
| `switch -- a {a{set m yes} ...}` | rc 1, `extra switch pattern with no body` | same; script implicit word joining does not apply inside the switch list |
| `trace add variable q read {list}` | accepted, rc 0 | accepted, rc 0 |

The current codebase comparison exposes two concrete modelling corrections. First, `BigIpExecutionContext` has no iCall variant even though the measured iCall surface is real and separately invokable; it must not be silently treated as iApp merely because this build produced identical rows. Second, current documentation/runtime comments mark bare `matches` semantics and `not` precedence as unmeasured; this run measures both discriminators above. The APL presentation DSL and Tcl callbacks embedded in APL remain unmeasured and must not inherit the implementation/iCall results.

The execution mechanisms follow F5's primary documentation for [iRule `call`](https://clouddocs.f5.com/api/irules/call.html), [tmsh CLI scripts](https://clouddocs.f5.com/cli/tmsh-reference/v16/modules/cli/cli_script.html), [iApp templates](https://clouddocs.f5.com/cli/tmsh-reference/v16/modules/sys/sys_application_template.html), [iCall scripts](https://clouddocs.f5.com/cli/tmsh-reference/v15/modules/sys/sys_icall_script.html), [iCall events](https://clouddocs.f5.com/cli/tmsh-reference/latest/modules/sys/sys_icall_event.html), and [triggered handlers](https://clouddocs.f5.com/cli/tmsh-reference/v16/modules/sys/sys_icall_handler_triggered.html). Raw results are [TMM LTM](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006l/ltm-raw.log`), [tmsh](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006l/cli-run.txt), [iApp and iCall](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006l/scriptd-results.txt), [tclsh8.4](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006l/host-tclsh8.4.txt`), and [tclsh8.5](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006l/host-tclsh8.5.txt`).


## Exact evidence

[bigip-hosted-context-parser-matrix.json](bigip-hosted-context-parser-matrix.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.

```json
[]
```
