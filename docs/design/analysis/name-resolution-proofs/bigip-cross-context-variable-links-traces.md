# naming.bigip.cross-context-variable-links-traces

Kind: `native-observation`

## Problem statement

Scalar/array cells, alias links and trace callbacks can receive differently formed names. A display-only subject or callback-name inference could change which cell is accessed.

## Question

What do the exact variable/array/link/trace controls return in each recorded execution context?

## Conclusion

The retained matrix records results and rejected doors for each context independently. Counted names, callback arguments and alias linkage are observable program facts; they supply neither a source editor span nor an independently entered native frame.

## Scope

Recorded context programs only; unavailable operations and failed setup remain explicit and no cross-context capability is borrowed. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

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

The retained matrix records results and rejected doors for each context independently. Counted names, callback arguments and alias linkage are observable program facts; they supply neither a source editor span nor an independently entered native frame.

The measured report rows and their limits are:


- Scalar names, array roots, and array indexes are counted byte sequences in
  all reached contexts. Embedded NUL did not truncate or alias a longer name.
- Qualified roots under a dynamically created ASCII namespace behaved the same
  way. This does not establish support for NUL inside a namespace component.
- `upvar` selected and mutated the intended NUL-bearing scalar, array root, or
  array element. The sibling short name retained its previous value.
- Direct array-root traces received the complete root bytes and index `6b`.
  Direct index traces received the ASCII root and index `41` or `410042`.
- A write through an array-root alias selected the counted original array but
  the callback's `name1` was hex `61727261795f616c696173` (`array_alias`), not
  the original qualified root. Selected cell identity and callback spelling
  must therefore remain separate compiler concepts.
- The element-alias mutation succeeded, but the root trace did not emit a
  distinct alias-spelled write row for that mutation. Later reads observed the
  changed counted element and supplied the original root plus `410042` index.
- The canonical non-TMM trace failures were exactly `invalid command name
  "TMM::cmp_group"`. The portable wrappers removed only that TMM-specific log
  call and retained all naming operations and callback byte capture.


## Exact evidence

[bigip-cross-context-variable-links-traces.json](bigip-cross-context-variable-links-traces.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.

```json
[]
```
