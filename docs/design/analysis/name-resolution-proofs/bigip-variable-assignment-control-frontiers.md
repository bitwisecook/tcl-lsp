# naming.bigip.variable-assignment-control-frontiers

Kind: `native-observation`

## Problem statement

A multiple-destination operation can fail while setting its first receiver, before an intended later rebinding discriminator. Another earlier case can also contaminate a shared frame. Those unreached states cannot be treated as observed assignment semantics.

## Question

Which assignment/getter lifecycle controls reached their intended stage in the61-case matrix?

## Conclusion

The exact matrix retains45successful assignment/getter programs across five name kinds, five unavailable lassign doors, and sequential-scan first-set failures that did not reach rebinding. The quiet-missing local fixture is explicitly contaminated by a prior value and proves no missing-cell result.

## Scope

All61cases on each of four actual TMMs; exact name/result bytes and setup/callback stages are retained, including counterexamples. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Channel: All61cases on each of four actual TMMs; exact name/result bytes and setup/callback stages are retained, including counterexamples. Version: bigip 21.1.0.1.

The exact matrix retains45successful assignment/getter programs across five name kinds, five unavailable lassign doors, and sequential-scan first-set failures that did not reach rebinding. The quiet-missing local fixture is explicitly contaminated by a prior value and proves no missing-cell result.

The measured report rows and their limits are:

The full matrix contains 61 cases on each of the four actual TMMs. After
removing only the group/unit columns, all four decoded matrices have SHA-256
`117f2950bdaaf49a947ba687d9b144bfd417b4bb3430e972d0712f37cd7d8d63`.
The complete rows are retained as
`vl-controls-42100.tsv`
and the peer `42101`–`42103` files.

| Cases | TMM coverage | Measured result |
| --- | --- | --- |
| active scalar | `0:0`–`0:3` | `{NEW EXTRA} {NEW EXTRA} {read unset} {}` |
| active array element | `0:0`–`0:3` | Same value and callback order as scalar; supplemental callback tuples were `{read 61 6b}` then `{unset 61 6b}`, proving root `a`, index `k` |
| older pending read chain | `0:0`–`0:3` | `{NEW EXTRA} {NEW EXTRA} {kill-read kill-unset} {}`; the earlier `later` read registration did not run after the active callback destroyed the traced cell |
| recursive ordinary read/write | `0:0`–`0:3` | `{INNER EXTRA} {INNER EXTRA} {read write}` |
| `lset` receiver recreation | `0:0`–`0:3` | inner catch 0; answer and recreated receiver both `{NEW EXTRA}` |
| nonempty `lappend` recreation | `0:0`–`0:3` | inner catch 0; answer and recreated receiver both `{NEW EXTRA}` |
| zero-addition `lappend` recreation | `0:0`–`0:3` | inner catch 0; answer and recreated receiver both `NEW` |
| callback error during quiet read | `0:0`–`0:3` | inner catch 0; `{answer errorCode value}` was `{} SENTINEL {}` |
| local quiet-missing fixture | `0:0`–`0:3` | Not a valid missing-cell observation: a prior case left `v` as `{NEW EXTRA}` in the shared event frame; retained as a counterexample |
| `append`, `lappend`, zero-addition `lappend`, `lset`, `scan`, `binary scan`, `catch` output, fail-index output, `info default` | `0:0`–`0:3`; plain, qualified, array-index, binary-NUL and decomposed names | All 45 programs reached their expected assignment/getter step with inner catch 0; exact name/result bytes are in the TSV and original/adapted fixture trees |
| `lassign` | `0:0`–`0:3`; all five name kinds | All five returned inner catch 1, exact message `invalid command name "lassign"`; no assignment semantics inferred |
| sequential `scan`, `upvar #0` | `0:0`–`0:3` | inner catch 1, `couldn't set variable "first"` |
| sequential `scan`, `upvar 0` | `0:0`–`0:3` | same rejection; callback attempted to turn the already-existing local `second` into a link, so the intended second-destination receiver discriminator was not reached |

The supplemental callback-argument responses are
`vl-argument-bytes-42100.body`
through `42103`; all returned operation/export catch 0 and identical result
hex.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md)
