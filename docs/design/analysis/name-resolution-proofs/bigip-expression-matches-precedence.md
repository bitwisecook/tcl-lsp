# naming.bigip.expression-matches-precedence

Kind: `native-observation`

## Problem statement

The pair comparing a literal pattern to itself cannot distinguish equality from glob matching. Independently, prefix not can bind before a following string predicate. Both ambiguities affect expression consumers.

## Question

What do the nine exact matches/not discriminators return in TMM, CLI script, iApp implementation and iCall?

## Conclusion

For the recorded387-byte program, bare matches is whole-string glob matching: a* matches abcd, while a.* and bc do not. not binds to its operand before starts_with unless parentheses group the predicate. All four recorded hosted contexts return the same nine rows; APL remains untested.

## Scope

Exact nine-row source and four hosted channels; the initial shorter matrix remains insufficient by itself and no other release is inferred. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

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

For the recorded387-byte program, bare matches is whole-string glob matching: a* matches abcd, while a.* and bc do not. not binds to its operand before starts_with unless parentheses group the predicate. All four recorded hosted contexts return the same nine rows; APL remains untested.

The measured report rows and their limits are:


The exact 387-byte expression payload ran in an `HTTP_REQUEST` iRule, a tmsh CLI script, an iApp implementation action and a triggered iCall script. All four BIG-IP-hosted contexts returned the same result bytes:

| Expression | Catch | Result |
| --- | ---: | --- |
| `"abcd" matches "a*"` | 0 | 1 |
| `"a*" matches "a*"` | 0 | 1 |
| `"a*" matches "abcd"` | 0 | 0 |
| `"abcd" matches "a.*"` | 0 | 0 |
| `"abcd" matches "bc"` | 0 | 0 |
| `"abcd" matches "abcd"` | 0 | 1 |
| `1 or 0 matches 0` | 0 | 1 |
| `not "abc" starts_with "a"` | 1 | `can't use non-numeric string as operand of "!"` |
| `not ("abc" starts_with "a")` | 0 | 0 |

For these discriminators, bare `matches` is whole-string glob matching: `a*` matches `abcd`, while `a.*` and `bc` do not. The earlier equality/non-substring pair alone was insufficient to distinguish equality from glob behavior. `not` binds to the following operand before `starts_with` unless parentheses group the string predicate.

The non-TMM fixtures loaded without error, the tmsh script emitted directly, the iApp emitted from its implementation action, and the iCall emitted from a triggered handler in `scriptd`. Raw outputs are [tmsh](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/contexts/cli-run.txt`) and [iApp/iCall](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/contexts/scriptd-results.txt`). iApp presentation/APL Tcl was not tested and must not inherit the implementation result.


## Exact evidence

[bigip-expression-matches-precedence.json](bigip-expression-matches-precedence.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.

```json
[]
```
