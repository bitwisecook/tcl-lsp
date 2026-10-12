# BIG-IP lexical byte-class results template

Execution branch: **work**. Reference target: **BIG-IP 21.1.0.1, build 0.0.26,
Point Release 1**. Template status: **NOT EXECUTED / no peer response**.
Populate the actual execution response in `LEXICAL_BYTE_CLASS_RESULTS.md`.
Use [LEXICAL_BYTE_CLASS_CHECKS.md](LEXICAL_BYTE_CLASS_CHECKS.md) without changing
its byte producers. Record the actual product/build; the reference version
does not identify an executed target.

## Existing measured gap

The existing [context result](CONTEXT_NAMING_RESULTS.md#lexical-name-boundary)
measured TMM stopping before `e9`, `c3`, `cc`, and non-TMM contexts consuming `e9`
and stopping after `c3`/`cc` but before `a9`/`81`. Braced controls consumed the
complete tested names. Full byte-class behavior is unmeasured. The measured
`%c 769` result is byte `01`, independently of binary `cc81`; neither observation
establishes a Unicode class or an unmeasured product's behavior.

## Run identity and source closure

| Field | Required observation |
| --- | --- |
| Exact `work` commit | PENDING |
| Product / version / build / edition | PENDING full raw version output |
| Context Tcl/package versions | PENDING, kept separately |
| Payload SHA-256 before/after transfer | PENDING |
| Full iRule SHA-256 before/after transfer | PENDING |
| CLI / iApp / iCall / optional APL wrapper hashes | PENDING |
| Fixture archive and appliance-side checksum verification | PENDING |
| Exact raw capture inventory and archive SHA-256 | PENDING |
| Any independently hashed capture-only repair | NONE REPORTED |

Expected checked-source hashes (not executed evidence): payload `af1d6366e57927bc72cd756bf8804e1a1713de958561f30258876603bc120ae9`;
complete iRule `49b3c8dcd43221031288595afc53f9cc4642fb714e5a57b123b15d3613d91a40`.

## Real backend and reached execution

| Context | Actual execution proof | Complete 1,044 rows | Outer export status | Exact result hash | Unsupported boundary |
| --- | --- | --- | --- | --- | --- |
| TMM HTTP_REQUEST, each observed unit | PENDING full raw HTTP response/session | PENDING | PENDING | PENDING | PENDING |
| TMM CLIENT_ACCEPTED, each observed unit | PENDING complete LTM chunk set | PENDING | PENDING | PENDING | PENDING |
| TMM RULE_INIT, each observed unit | PENDING complete LTM chunk set | PENDING | PENDING | PENDING | PENDING |
| tmsh CLI script | PENDING executed script/stdout/stderr/status | PENDING | PENDING | PENDING | PENDING |
| iApp implementation | PENDING actual implementation action output | PENDING | PENDING | PENDING | PENDING |
| triggered iCall | PENDING delivered execution/output | PENDING | PENDING | PENDING | PENDING |
| APL, if actually executed | NOT EXECUTED | UNKNOWN | UNKNOWN | UNKNOWN | No substitution from another context |

Record dev.bragi0.com's real server loopback/reachable-interface baseline,
raw VIP pass-through request/response, backend-observed peer, SNAT automap,
TCP/HTTP profiles, CMP and exact actual TMM identities. Preserve missing units
and incomplete/conflicting syslog chunks. A synthetic probe response or a
configuration listing alone cannot establish backend traffic or event coverage.
Use one initialization and the finite one-client-per-reached-unit plan in the
request. Attach each batch's full first/last sequence, expected/received count,
SHA-256 and 1,044-row key coverage. Do not fill missing TMMs by a hot logging
sweep; incomplete or truncated event rows remain unknown. Hash any capture-only
batching wrapper separately and preserve every tested producer/source/eval word.

## Row-level results

Attach all original rows before adding any summary. Raw cases 0..255 map to
singleton binary input bytes 00..ff; cases 256..260 are `%c 233`, `%c 769`, binary
`c3a9`, binary `e9`, binary `cc81`. Keep `append_source`/`binary_source` and
unbraced/braced separate. `constructor_hex` is the binary construction input;
post-eval H* and separately caught UTF-8 diagnostics are observed conversions,
not interchangeable descriptions of the original source object's storage.

| Actual context / version / TMM | Case / factory / form / source kind | Source build + constructor hex | Source scan status + hex | Eval catch code | Result scan status + hex | Exact read callback sequence | Setup / trace / cleanup boundaries |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PENDING | PENDING | PENDING | PENDING | PENDING | PENDING | PENDING | PENDING |

## Conclusion limits

No new result is presently available. When populated, distinguish a source
producer refusal, a parser rejection, a reached read, a guest command error and
a report-conversion failure. `NOT_REACHED` supplies no byte classification.
Read trace absence supplies no result when the trace command refused. Preserve
all 256 rows, including delimiters and syntax errors, without replacing them
with guessed ASCII/Unicode/ctype categories. Do not infer NameTable identity,
compiler selection, cache/lifetime ownership or broader TMM versus non-TMM
behavior from a lexical observation. State exactly which independently selected
context/version/source-producer classes the new evidence can close.
