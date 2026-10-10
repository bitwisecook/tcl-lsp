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

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: All61cases on each of four actual TMMs; exact name/result bytes and setup/callback stages are retained, including counterexamples.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

The exact matrix retains45successful assignment/getter programs across five name kinds, five unavailable lassign doors, and sequential-scan first-set failures that did not reach rebinding. The quiet-missing local fixture is explicitly contaminated by a prior value and proves no missing-cell result.

The measured report rows and their limits are:


The full matrix contains 61 cases on each of the four actual TMMs. After
removing only the group/unit columns, all four decoded matrices have SHA-256
`117f2950bdaaf49a947ba687d9b144bfd417b4bb3430e972d0712f37cd7d8d63`.
The complete rows are retained as
[`vl-controls-42100.tsv`](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/decoded/vl-controls-42100.tsv)
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
[`vl-argument-bytes-42100.body`](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-argument-bytes-42100.body)
through `42103`; all returned operation/export catch 0 and identical result
hex.


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md). SHA-256 `469555b59e24291fa07e50a5aaf531ff13034270a20d44cdde4ad4b01cb833d8`. Lines 117–146. Exact build-specific measured report section; all rows and limits remain in the retained report.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md). SHA-256 `469555b59e24291fa07e50a5aaf531ff13034270a20d44cdde4ad4b01cb833d8`. Lines 11–11. Actual appliance version attribution for this report; no separately named hotfix inferred.
- `raw-0` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-argument-bytes-42100.body](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server/vl-argument-bytes-42100.body). SHA-256 `5cf1fed10ea783f73a96d3dd3bb72e332372a4f8831cb62fbab1689f0a370c0a`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `raw-1` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/decoded/vl-controls-42100.tsv](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/decoded/vl-controls-42100.tsv). SHA-256 `6f40fcec1fb43b1c01a79ae0b39393233ef21538c9401261bbfc98bc5d671889`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `source-closure-4` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/r2286vl1-fixtures-v4/manifest.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/appliance/r2286vl1-fixtures-v4/manifest.json). SHA-256 `5d926a45e3efa7916ca5ef8af3800fe9516d5ce5c9af74401854584649fccbbf`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
