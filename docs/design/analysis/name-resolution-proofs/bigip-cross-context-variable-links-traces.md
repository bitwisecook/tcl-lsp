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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Channel: Recorded context programs only; unavailable operations and failed setup remain explicit and no cross-context capability is borrowed. Version: bigip 21.1.0.1.

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

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md)
