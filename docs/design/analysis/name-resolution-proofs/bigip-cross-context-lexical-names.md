# naming.bigip.cross-context-lexical-names

Kind: `native-observation`

## Problem statement

A namespace/cell may retain a full byte name while an unbraced variable scanner consumes only a prefix. Applying runtime name-table rules to lexical tokens would misidentify the read.

## Question

Where do the measured braced/unbraced lexical variable forms stop in the recorded contexts?

## Conclusion

The retained context matrix distinguishes full braced name reads from unbraced lexical stopping and preserves their exact errors/results. Source-produced names and runtime name lookup remain separate axes; the dedicated complete-byte scan record supplies the exhaustive finite singleton controls.

## Scope

Only exact context matrix sources/forms and reached rows; no native header layout or untested Unicode class is inferred. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Channel: Only exact context matrix sources/forms and reached rows; no native header layout or untested Unicode class is inferred. Version: bigip 21.1.0.1.

The retained context matrix distinguishes full braced name reads from unbraced lexical stopping and preserves their exact errors/results. Source-produced names and runtime name lookup remain separate axes; the dedicated complete-byte scan record supplies the exhaustive finite singleton controls.

The measured report rows and their limits are:

The clean payload seeded the ASCII prefix cell with `SHORT_PREFIX` and the
complete dynamic cell with `FULL_VALUE`, then evaluated one substitution only.

| Runtime suffix bytes | TMM unbraced result | tmsh/iApp/iCall unbraced result | Braced result in all contexts |
| --- | --- | --- | --- |
| `%c 233` -> `e9` | `SHORT_PREFIX` + `e9` | `FULL_VALUE` | `FULL_VALUE` |
| `%c 769` -> `01` | `SHORT_PREFIX` + `01` | `SHORT_PREFIX` + `01` | `FULL_VALUE` |
| binary `c3a9` | `SHORT_PREFIX` + `c3a9` | catch 1; attempted name ended in `c3` | `FULL_VALUE` |
| binary `cc81` | `SHORT_PREFIX` + `cc81` | catch 1; attempted name ended in `cc` | `FULL_VALUE` |
| binary `0042` | `SHORT_PREFIX` + `0042` | `SHORT_PREFIX` + `0042` | `FULL_VALUE` |

Therefore a centralized compiler interface needs a context-specific lexical
name scanner. Dynamic name identity is byte-preserving in these controls, but
unbraced source-token consumption is not the same in TMM and scriptd/tmsh Tcl.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md)
