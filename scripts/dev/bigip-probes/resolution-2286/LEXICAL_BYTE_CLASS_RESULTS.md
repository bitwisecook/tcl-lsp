# BIG-IP lexical byte-class results

These are measured results for the complete 1,044-row matrix requested by
[`LEXICAL_BYTE_CLASS_CHECKS.md`](LEXICAL_BYTE_CLASS_CHECKS.md). The complete
matrix was recovered from real TMM HTTP_REQUEST execution, an actual tmsh CLI
script, an invoked iApp implementation action, and a delivered triggered iCall
event. Standalone Tcl was not substituted for any BIG-IP context.

## Appliance, source and topology

| Property | Measured value |
| --- | --- |
| Product | BIG-IP 21.1.0.1 |
| Build / edition / hotfix | build 0.0.26, Point Release 1; no separate hotfix shown |
| Build date | Tue Jul 14 05:03:24 PDT 2026 |
| FIPS | Cryptographic Module for BIG-IP |
| Platform | BIG-IP Virtual Edition, Z100 |
| Topology | standalone, active |
| TMM process and actual roster | PID 16500; `0:0`, `0:1`, `0:2`, `0:3` |
| Tested source commit | `88198d83d3ef944670d7a1f24a3ada1b11187f69` on `work` |
| VIP / backend | `192.168.9.24:18860` / `192.168.9.80:18760` |
| Profiles / translation | TCP and HTTP; SNAT automap |
| Context Tcl observation | Existing context payload on this appliance/build measured Tcl 8.4.6 / package Tcl 8.4 in TMM, tmsh, iApp and iCall; this lexical wrapper did not repeat the environment payload |

Complete `tmsh show sys version` output:

```text
Sys::Version
Main Package
  Product      BIG-IP
  Version      21.1.0.1
  Build        0.0.26
  Edition      Point Release 1
  Date         Tue Jul 14 05:03:24 PDT 2026
  FIPS Module  Cryptographic Module for BIG-IP
```

The exact TMM and platform outputs are
[`system-tmm-info.txt`](evidence/r2286_20261007vllex/appliance/system-tmm-info.txt)
and
[`system-hardware.txt`](evidence/r2286_20261007vllex/appliance/system-hardware.txt).

## Byte provenance and load

The generator extracted all three Tcl blocks directly from the request
Markdown and refused unexpected hashes. Source was ASCII/LF and contained no
literal NUL or non-ASCII configuration byte. Every tested NUL/non-ASCII value
was produced at runtime. `%c` results, binary-produced bytes, source scans and
post-eval UTF-8 diagnostics remain distinct fields in every retained row.

| Item | SHA-256 | Bytes |
| --- | --- | ---: |
| Common payload | `af1d6366e57927bc72cd756bf8804e1a1713de958561f30258876603bc120ae9` | 7,698 |
| Complete supplied iRule body | `49b3c8dcd43221031288595afc53f9cc4642fb714e5a57b123b15d3613d91a40` | 27,379 |
| Full iRule configuration | `da70a3b4486699dbf283f19c4db22077bf0f181c3c269c398c78e4a960e96ed8` | 27,429 |
| tmsh CLI wrapper | `81a7dc66e3800cc8f629e064e78ff85d7601fd840fe753e9557f3ce8b837a181` | 8,234 |
| iApp wrapper | `d532685039d99d976e7b2fe3134e839e065ffaec52fa5b9d73f50d2c1505221d` | 8,335 |
| iCall wrapper | `5d2d3989a36bbb58899ae413e0faba9ce72b56053eda64c5e6c54880610926e4` | 8,441 |
| Executed fixture archive | `d67e6c917a224468ec18d4a07eda37d660cefa90f792b050095a236de3da9547` | retained |
| Executed fixture manifest | `bd2ad1e4bd24adf5611e02dfa4ba9872bc9c52b0bd04bf4ba4b4f5f27c701253` | retained |

The appliance-side verification and full source tree are
[`r2286lex1-v2-sha256-check.txt`](evidence/r2286_20261007vllex/appliance/r2286lex1-v2-sha256-check.txt)
and
[`r2286lex1-fixtures-v2`](evidence/r2286_20261007vllex/appliance/r2286lex1-fixtures-v2/).

One provenance field is explicitly not authoritative: the executed v2
manifest's caller-supplied `source_commit` string is
`88198d83da0a239036da06386846699195f30220`, while the measured worktree HEAD
was `88198d83d3ef944670d7a1f24a3ada1b11187f69`. A fresh generation at the
measured HEAD differs only in `manifest.json`; every payload, wrapper and lab
file in `SHA256SUMS` is byte-identical. The request Markdown hash is
`c8d1fcd88e14d8b2a61901b7250ee6c5b28cafc4db17c51ec231cd385e64595e`.
The source-byte hashes above, rather than the incorrect free-form manifest
field, close fixture provenance.

The exact iRule loaded with status 0 and twelve warnings of the form:

```text
warning: [variable reference used where variable name expected][$r2286_lbc_name]
```

All warning locations and loader text are retained in
[`lex-load-irule.stdout`](evidence/r2286_20261007vllex/appliance/lex-load-irule.stdout).
No tested producer, constructed source word or eval word was repaired.

## Backend, CMP and actual execution coverage

The direct reachable backend baseline and pool-backed baseline succeeded. The
same heavy connection from client source port 43100 first passed
`/r2286-backend-pass` through the pool, then requested the probe. The backend
recorded translated peer `192.168.9.24`, request marker `lexical-43100-0`, and
returned through the VIP. Raw bytes are
[`lex-heavy-u0-0.request`](evidence/r2286_20261007vllex/client-server/lex-heavy-u0-0.request),
[`lex-heavy-u0-0.body`](evidence/r2286_20261007vllex/client-server/lex-heavy-u0-0.body)
and the backend log in the same directory.

Before attachment, the lightweight identity rule mapped source ports 43100,
43101, 43102 and 43103 to `0:0`, `0:1`, `0:2` and `0:3`. After attachment, the
exact heavy rule changed the observed virtual to `CMP Mode : single-cpu`.
Consequently, all four allowed heavy connections actually executed on `0:0`.
No extra heavy connections were used to manufacture multi-TMM coverage. The
identity map, virtual status and four response headers are retained as raw
evidence.

| Context | Actual execution | Complete rows | Raw export SHA-256 | Decoded matrix SHA-256 |
| --- | --- | ---: | --- | --- |
| TMM HTTP_REQUEST | four bounded connections, all actual `0:0`; authoritative response body identical | 1,044 unique keys | `bedfc0132b718a2817282f22a8543230728acf0442f05c78b15d244a69e97e29` | `9673c3a3f3c25e7e4e4393f8cb20c7934fed79daaf2537453f33320ddc887042` |
| tmsh CLI | `tmsh run cli script`; status 0, empty stderr | 1,044 unique keys | `38256d70d919ca5633c3d503d29338fbc44bdb81f42daf4ef573aa8c8751d8ec` | `d709338fec0ae6828ec479a6d747d245844549c2a0918bda9fc9121ae9c62e8f` |
| iApp implementation | `tmsh create sys application service ... template ...`; actual action output isolated by scriptd byte offsets | 1,044 unique keys | `e241c18d855afefe5f5e43d6eef88f6dfd43d37b9f236ccd174e5f88250e08e4` | `335b51575c8159bc4b896a8543d29d953413dae918593c4712fa8bc36cce1d01` |
| triggered iCall | handler loaded, event delivered by `generate sys icall event`; status 0 | 1,044 unique keys | `67cf7eb75f769b33cea9743523e7509cac565b3ad0974b3103be8f8d81d59df8` | `408169a9163e4db5e649a840c9112ae344f783634dfff85b8563f2515994c052` |
| APL | not executed; no presentation evaluator/output mechanism was established | unknown | none | none |

After removing only context and identity columns, the tmsh, iApp and iCall
matrices are byte-identical with SHA-256
`04adeb7097ef5d22db36ab3dbc0294b1027e60f84ec20100e0f9eda0186cd78f`.
The TMM matrix differs.

The complete row-level results—not a sample—are
[`http-matrix.tsv`](evidence/r2286_20261007vllex/decoded/http-matrix.tsv),
[`cli-matrix.tsv`](evidence/r2286_20261007vllex/decoded/cli-matrix.tsv),
[`iapp-matrix.tsv`](evidence/r2286_20261007vllex/decoded/iapp-matrix.tsv) and
[`icall-matrix.tsv`](evidence/r2286_20261007vllex/decoded/icall-matrix.tsv).
Each has 1,045 lines including its header and retains the complete raw row as
hex in its last column.

## Complete matrix summary

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

## RULE_INIT and CLIENT_ACCEPTED limits

The exact rule executed one RULE_INIT on every actual TMM and one
CLIENT_ACCEPTED for each of the four bounded connections. Each event declared
6,444 chunks. Continuous syslog capture dropped chunks under this volume, so
none is accepted as a complete transcript:

| Context / actual unit or session | Unique chunks | Missing | Observed range | Result |
| --- | ---: | ---: | --- | --- |
| RULE_INIT `0:0` | 885 / 6,444 | 5,559 | 1536–6443 | incomplete / rows unknown |
| RULE_INIT `0:1` | 885 / 6,444 | 5,559 | 1610–6443 | incomplete / rows unknown |
| RULE_INIT `0:2` | 178 / 6,444 | 6,266 | 0–6050 | incomplete / rows unknown |
| RULE_INIT `0:3` | 139 / 6,444 | 6,305 | 5533–6216 | incomplete / rows unknown |
| CLIENT_ACCEPTED `43100` / `0:0` | 2,643 / 6,444 | 3,801 | 0–6440 | incomplete / rows unknown |
| CLIENT_ACCEPTED `43101` / `0:0` | 3,033 / 6,444 | 3,411 | 0–5832 | incomplete / rows unknown |
| CLIENT_ACCEPTED `43102` / `0:0` | 2,383 / 6,444 | 4,061 | 0–6413 | incomplete / rows unknown |
| CLIENT_ACCEPTED `43103` / `0:0` | 2,893 / 6,444 | 3,551 | 0–6425 | incomplete / rows unknown |

The authoritative finite inventory is
[`lex-log-chunk-inventory.json`](evidence/r2286_20261007vllex/decoded/lex-log-chunk-inventory.json).
No partial stream was decoded, no reload was used to fill RULE_INIT, and no
connection beyond the request's four-connection limit was generated.

## Reproduction

```sh
python3 generate-lexical-byte-class-fixtures.py \
  --out /tmp/r2286lex1-fixtures --source-commit 88198d83d3ef944670d7a1f24a3ada1b11187f69
(cd /tmp/r2286lex1-fixtures && sha256sum -c SHA256SUMS)
tmsh load sys config merge file sources/identity.conf
tmsh load sys config merge file sources/lab.conf
tmsh load sys config merge file sources/irule.conf
tmsh modify ltm virtual /Common/__tcl_lsp_2286_r2286lex1_vs \
  rules '{' /Common/__tcl_lsp_2286_r2286lex1_lbc '}'
```

The retained client keeps the backend pass and probe on one connection:

```sh
python3 http_keepalive_probe.py --host 192.168.9.24 --port 18860 \
  --local-port 43100 --output-prefix lex-heavy-u0 \
  /r2286-backend-pass /r2286-lexical-byte-class
tclsh decode-lexical-byte-class.tcl lex-heavy-u0-1.body \
  http-matrix.tsv http-summary.txt
```

Non-TMM execution used:

```sh
tmsh run cli script /Common/__tcl_lsp_2286_r2286lex1_cli
tmsh create sys application service \
  /Common/__tcl_lsp_2286_r2286lex1_iapp_service \
  template /Common/__tcl_lsp_2286_r2286lex1_iapp
tmsh generate sys icall event name R2286LEX1_LBC
```

## Evidence and cleanup

The retained evidence bundle is
[`evidence/r2286_20261007vllex`](evidence/r2286_20261007vllex/), with 860
files inventoried in
[`SHA256SUMS`](evidence/r2286_20261007vllex/SHA256SUMS). The continuous raw LTM
capture is
[`ltm-continuous.log`](evidence/r2286_20261007vllex/appliance/ltm-continuous.log).
Exact requests, headers, bodies and backend observations are under
[`client-server`](evidence/r2286_20261007vllex/client-server/).

All owned virtuals, pools, rules, tmsh scripts, iApp templates/services, iCall
objects and the disposable data-group control were removed. The post-cleanup
owned-object inventories are empty. The pre-existing shared backend node was
preserved. Owned backend and continuous-capture processes were stopped and
verified absent. Raw evidence directories and reusable harness source remain
available by design.

These conclusions apply only to the measured byte constructors, source-object
builders, forms, contexts and BIG-IP build. They do not establish NameTable
identity, normalization, locale/ctype ownership, cache lifetime, command-name
scanning, or behavior in APL and the incomplete TMM events.
