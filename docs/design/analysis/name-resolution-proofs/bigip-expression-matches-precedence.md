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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Channel: Exact nine-row source and four hosted channels; the initial shorter matrix remains insufficient by itself and no other release is inferred. Version: bigip 21.1.0.1.

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

The non-TMM fixtures loaded without error, the tmsh script emitted directly, the iApp emitted from its implementation action, and the iCall emitted from a triggered handler in `scriptd`. Raw outputs are tmsh and `iApp/iCall`. iApp presentation/APL Tcl was not tested and must not inherit the implementation result.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md)
