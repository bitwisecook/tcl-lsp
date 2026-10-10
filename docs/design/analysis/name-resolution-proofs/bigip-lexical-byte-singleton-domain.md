# naming.bigip.lexical-byte-singleton-domain

Kind: `native-observation`

## Problem statement

Unbraced variable parsing may use a byte class that differs between TMM and hosted non-TMM contexts. Treating apparent Unicode text as the parser domain or using callback counts as success would classify the wrong tokens.

## Question

Which of the256singleton bytes and five explicit producer controls are consumed in each measured unbraced/braced form?

## Conclusion

TMM consumes the63ASCII digit/letter/underscore bytes; tmsh/iApp/iCall consume those plus the recorded high-byte ranges for128bytes. Syntax-bearing error cases are separate. Explicit format/binary products and the binary-source braced257 counterexample are preserved; these rows establish neither a Unicode class nor internal representation.

## Scope

Four complete contexts, two source builders,1044source-build/eval rows per matrix and exact callbacks; RULE_INIT/CLIENT_ACCEPTED incomplete controls are excluded from complete-matrix claims. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

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

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: Four complete contexts, two source builders,1044source-build/eval rows per matrix and exact callbacks; RULE_INIT/CLIENT_ACCEPTED incomplete controls are excluded from complete-matrix claims.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

TMM consumes the63ASCII digit/letter/underscore bytes; tmsh/iApp/iCall consume those plus the recorded high-byte ranges for128bytes. Syntax-bearing error cases are separate. Explicit format/binary products and the binary-source braced257 counterexample are preserved; these rows establish neither a Unicode class nor internal representation.

The measured report rows and their limits are:


All four complete contexts produced 1,033 eval catch code 0 rows and eleven
catch code 1 rows. All 1,044 source-build operations completed. The following
is a classification of those retained rows, not a replacement for them.

### Unbraced singleton scan

For cases 0–255, a `FULL` result means the tested singleton and trailing `Q`
were consumed into the seeded complete name. Both `append_source` and
`binary_source` produced the same measured classification.

| Context family | Singleton bytes consumed by the unbraced name scanner |
| --- | --- |
| TMM HTTP_REQUEST | `30-39`, `41-5a`, `5f`, `61-7a` (63 bytes: ASCII digits, letters and underscore) |
| tmsh / iApp / iCall | `30-39`, `41-5a`, `5f`, `61-7a`, `aa`, `b5`, `ba`, `c0-d6`, `d8-f6`, `f8-ff` (128 bytes) |

This is a measured byte-domain result. It does not establish a Unicode class,
locale owner, compiler choice or internal string representation.

The five unbraced syntax/error rows were the same for each source producer:

| Case / byte | Exact eval result |
| --- | --- |
| 10 / `0a` | catch 1, `invalid command name "Q"` |
| 36 / `24` | catch 1, `can't read "Q": no such variable` |
| 40 / `28` | catch 1, `missing )` |
| 59 / `3b` | catch 1, `invalid command name "Q"` |
| 91 / `5b` | catch 1, `missing close-bracket` |

These are syntax-bearing counterexamples, not letter-class decisions.

### Explicit non-ASCII producers

| Producer | Observed payload bytes | TMM unbraced | tmsh / iApp / iCall unbraced | Braced control |
| --- | --- | --- | --- | --- |
| `format %c 233` | `e9` | stopped before `e9`: `SHORT` + `e9 51` | consumed complete name: `FULL` | `FULL` in all contexts |
| `format %c 769` | `01` | stopped before `01`: `SHORT` + `01 51` | same | append-source `FULL`; see binary-source exception below |
| binary `c3a9` | `c3 a9` | stopped before `c3`: `SHORT` + `c3 a9 51` | consumed `c3` only: `PREFIX_1` + `a9 51` | `FULL` in all contexts |
| binary `e9` | `e9` | stopped before `e9`: `SHORT` + `e9 51` | consumed complete name: `FULL` | `FULL` in all contexts |
| binary `cc81` | `cc 81` | stopped before `cc`: `SHORT` + `cc 81 51` | consumed `cc` only: `PREFIX_1` + `81 51` | `FULL` in all contexts |

`format %c 769` producing byte `01` is an observed appliance conversion, not
UTF-8 `cc81`. The `binary_source` braced case 257 was the eleventh catch-1 row
in every context: `can't read "...v\x01Q": no such variable`. Its source build
and scan both succeeded, but the seeded cell and constructed binary source did
not resolve to the same variable. That counterexample is retained without
normalizing either object. In both source producers, braced case 125 (byte
`7d`, the closing brace) succeeds with result bytes `53484f5254515c7d`
(`SHORTQ\}`), because the byte closes the variable substitution. The exact
counts are 260 append-source and 259 binary-source braced cases returning
`FULL`; case 125 and the binary-source case 257 are retained exceptions.

### Read and trace evidence

Each row independently records setup, trace-add, eval, callback tuples,
trace-remove and cleanup. A delimiter can cause parser failure after some
observable reads, so callback count is not used as a surrogate for eval
success. The complete callback names and byte scans are in the matrix files.
No row is classified from a failed source build or a refused trace install.


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/LEXICAL_BYTE_CLASS_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/LEXICAL_BYTE_CLASS_RESULTS.md). SHA-256 `0c8d890e456c537df1f10b077dd014dd085270c0946ec298581899046d3d45d6`. Lines 127–187. Exact build-specific measured report section; all rows and limits remain in the retained report.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/LEXICAL_BYTE_CLASS_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/LEXICAL_BYTE_CLASS_RESULTS.md). SHA-256 `0c8d890e456c537df1f10b077dd014dd085270c0946ec298581899046d3d45d6`. Lines 13–13. Actual appliance version attribution for this report; no separately named hotfix inferred.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
