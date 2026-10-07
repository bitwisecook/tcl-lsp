# BIG-IP variable lifecycle results

These are measured results from real TMM iRule execution on the active
standalone lab appliance. Standalone `tclsh`, tmsh scripts, iApps and iCall
were not substituted for TMM execution.

## Appliance, source and topology

| Property | Measured value |
| --- | --- |
| Product | BIG-IP 21.1.0.1 |
| Build / edition / hotfix | build 0.0.26, Point Release 1; no separate hotfix shown |
| Build date | Tue Jul 14 05:03:24 PDT 2026 |
| FIPS | Cryptographic Module for BIG-IP |
| Platform | BIG-IP Virtual Edition, Z100 |
| Topology | standalone, active |
| TMM process and roster | PID 16500; `0:0`, `0:1`, `0:2`, `0:3` |
| Tested source commit | `88198d83d3ef944670d7a1f24a3ada1b11187f69` on `work` |
| VIP / backend | `192.168.9.24:18861` / `192.168.9.80:18761`; ordinary-global VIP `192.168.9.24:18862` |
| Profiles / translation | TCP and HTTP; SNAT automap on both VIPs |
| CMP | configured yes; observed `all-cpus` for the lifecycle and ordinary-global VIPs |

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

The complete TMM listing is retained in
[`system-tmm-info.txt`](evidence/r2286_20261007vllex/appliance/system-tmm-info.txt).
The identity phase selected source ports 42100, 42101, 42102 and 42103 for
units 3, 2, 1 and 0 respectively. Every lifecycle traffic phase below recorded
the actual unit in its response or LTM marker. The pool-backed baseline reached
the server, whose raw response records peer `192.168.9.24`; this proves the
SNAT translation and return path rather than merely listing the configuration.
See
[`vl-vip-backend.body`](evidence/r2286_20261007vllex/client-server/vl-vip-backend.body)
and
[`vl-context-event-42100.body`](evidence/r2286_20261007vllex/client-server/vl-context-event-42100.body).

## Source and byte closure

The request block was extracted from
[`VARIABLE_LIFECYCLE_CHECKS.md`](VARIABLE_LIFECYCLE_CHECKS.md), not retyped.
All original generic-consumer programs were copied byte-for-byte. Event
adaptations and generated controls are separately named and hashed. Generated
configuration source is ASCII/LF with no literal NUL; runtime NUL and
decomposed names are produced by the request's Tcl expressions.

| Item | SHA-256 |
| --- | --- |
| Request Markdown | `25fca270d76f2d33a292bc01bcdf70525536df9a59f735e5d589d75dafb5128c` |
| Extracted request block | `9070692d96ad4ed5ce889d922a6049fe4a94fc140ee99a896e0890e51ae85230` |
| Token-substituted exact rule body | `c2de31571f7f7d5f0d08d27fa37bae3af8595060193dd5a352f22ad36cb3c2ee` |
| Exact literal wrapper | `f8c2fe92bf428f8948e34eb4844a840eb5dcfbaa305767372f811f60588816cf` |
| Supported runtime-eval wrapper | `55d51a96a7f0e4063373ed18e2736b43a44c9fbddc4906a5de0868d5ac0be893` |
| 61-case HTTP_REQUEST controls | `c26a2bef1b6893b7bd2a076329b7937470e5aea2a70c109b548f48f5949ee731` |
| `static::` rule | `f3a60b0859bc8fc2e7f2c4727dd3b5da270f2cf1c659e0125cdf7fdd33267097` |
| RULE_INIT/CLIENT_ACCEPTED wrapper | `d1e1884a2a722017cbe79513338ca5d9d59699b7df754f7a9dddc9de472ab90a` |
| Array callback-argument payload | `6bd4ddc2bac3d57610d1084032212844f38f3034777b6b122f1e6d85d0ba3878` |
| Array callback-argument rule | `bb8171639b0d74490fd6d13adcaa0f139727bd1a990551c32a18067b1d54b3cd` |
| Final fixture archive | `64a293b9500394b037ffd8d08242bc7971331ff495f18cf161f97bc316abc545` |

The final manifest is
[`manifest.json`](evidence/r2286_20261007vllex/appliance/r2286vl1-fixtures-v4/manifest.json),
and appliance-side `sha256sum -c` output is
[`vl-v4-verify.stdout`](evidence/r2286_20261007vllex/appliance/vl-v4-verify.stdout).
Earlier exact-load and supported-wrapper archives are retained in the same
evidence directory because their loader outcomes are part of the result.

## Literal load boundary

The exact literal main rule was rejected. This is a load-time result and is
not the runtime behavior reported below:

```text
01070151:3: Rule [/Common/__tcl_lsp_2286_vl1_main] error: /Common/__tcl_lsp_2286_vl1_main:7: error: [undefined procedure: R2286_vl1_hex][R2286_vl1_hex $name]
/Common/__tcl_lsp_2286_vl1_main:22: error: [wrong # args][trace add variable R2286_vl1_value {read unset} R2286_vl1_watch]
/Common/__tcl_lsp_2286_vl1_main:24: error: [undefined procedure: R2286_vl1_watch][R2286_vl1_watch]
/Common/__tcl_lsp_2286_vl1_main:33: error: [wrong # args][trace remove variable R2286_vl1_value {read unset} R2286_vl1_watch]

Unexpected Error: Loading configuration process failed.
```

The literal ordinary-global rule was independently rejected because `proc`
was not valid directly in event scope and the callbacks were consequently
undefined. Both supported wrappers retained the tested programs as hex-decoded
runtime source and loaded successfully. The exact errors and all loader status,
stdout and stderr files are under
[`appliance`](evidence/r2286_20261007vllex/appliance/).

## Main discriminator

Modern `trace add variable` and legacy `trace variable` each installed at
runtime with catch code 0 on every TMM. The operation also completed with code
0 and returned byte-identical results on `0:0` through `0:3`:

```text
{NEW EXTRA} {NEW EXTRA}
```

For both APIs, callback order was `read`, then `unset`. `name1` was exactly
hex `52323238365f766c315f76616c7565` (`R2286_vl1_value`), `name2` was empty,
and `trace info variable` was empty after recreation. The final read did not
invoke the destroyed registration. The raw per-unit responses are
[`vl-main-u0.body`](evidence/r2286_20261007vllex/client-server/vl-main-u0.body)
through
[`vl-main-u3.body`](evidence/r2286_20261007vllex/client-server/vl-main-u3.body).

## Control matrix

The full matrix contains 61 cases on each of the four actual TMMs. After
removing only the group/unit columns, all four decoded matrices have SHA-256
`117f2950bdaaf49a947ba687d9b144bfd417b4bb3430e972d0712f37cd7d8d63`.
The complete rows are retained as
[`vl-controls-42100.tsv`](evidence/r2286_20261007vllex/decoded/vl-controls-42100.tsv)
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
[`vl-argument-bytes-42100.body`](evidence/r2286_20261007vllex/client-server/vl-argument-bytes-42100.body)
through `42103`; all returned operation/export catch 0 and identical result
hex.

## Ordinary globals and CMP

The ordinary-global virtual remained `CMP Mode : all-cpus`, and the four
requests actually executed on units 0, 1, 2 and 3. This is direct response
coverage, not an inference from `cmp-enabled yes`.

| Case | Exact measured result |
| --- | --- |
| missing zero-addition `lappend` with `::errorCode` sentinel | `SENTINEL {}`; operation catch 0 |
| read callback raises `BOOM` | `{} NONE {}`; operation catch 0, showing the ordinary global error state changed to `NONE` in this control |
| sequential `scan` with global replacement | catch 1, `couldn't set variable "first"`; receiver rebinding was not reached for the same existing-local-link reason |

Raw responses are the four
[`vl-global-42100.body`](evidence/r2286_20261007vllex/client-server/vl-global-42100.body)
peer files. The installed configuration and observed CMP mode are in
[`vl-global-installed.conf`](evidence/r2286_20261007vllex/appliance/vl-global-installed.conf)
and
[`vl-global-virtual-show.txt`](evidence/r2286_20261007vllex/appliance/vl-global-virtual-show.txt).
These cases establish CMP eligibility and the recorded event behavior, not
cross-TMM sharing of an arbitrary ordinary global value.

## RULE_INIT, CLIENT_ACCEPTED and per-TMM `static::`

The representative active scalar control completed with identical result
bytes and `read unset` order in RULE_INIT once on each of `0:0`–`0:3`, then in
CLIENT_ACCEPTED on one mapped connection per unit. The complete direct markers
are in
[`vl-context-events.log`](evidence/r2286_20261007vllex/appliance/vl-context-events.log).

The `static::` rule logged a `SEED` initialization from every actual unit. The
following traffic then used the same four stable source tuples:

| Phase | `0:0` | `0:1` | `0:2` | `0:3` |
| --- | --- | --- | --- | --- |
| initial read | `SEED` | `SEED` | `SEED` | `SEED` |
| after write on `0:2` | `SEED` | `SEED` | `MARK_UNIT` | `SEED` |
| after unset/recreate on `0:2` | `SEED` | `SEED` | `RECREATED` | `SEED` |

The unset request on `0:2` observed catch 1 with exact message
`can't read "static::__tcl_lsp_2286_vl1_cell": no such variable` before
recreation. Thus the measured `static::` storage is per TMM for this rule and
version: neither mutation nor unset/recreation broadcast to the other three
units. The raw sequence is the `vl-static-*.body` set in
[`client-server`](evidence/r2286_20261007vllex/client-server/), and the four
initialization markers are in the continuous LTM capture.

## Reproduction

```sh
python3 generate-variable-lifecycle-fixtures.py \
  --out /tmp/r2286vl1-fixtures --source-commit 88198d83d3ef944670d7a1f24a3ada1b11187f69
(cd /tmp/r2286vl1-fixtures && sha256sum -c SHA256SUMS)
tmsh load sys config merge file sources/identity.conf
tmsh load sys config merge file sources/main-supported.conf
tmsh load sys config merge file sources/controls.conf
tmsh load sys config merge file sources/global.conf
tmsh load sys config merge file sources/static.conf
tmsh load sys config merge file sources/context-events.conf
tmsh load sys config merge file sources/argument-bytes.conf
tmsh load sys config merge file sources/lab.conf
```

Traffic was sent through the VIP with an explicitly bound client source port,
for example:

```sh
curl --local-port 42101 http://192.168.9.24:18861/r2286-vl1-static/read
tclsh decode-variable-lifecycle.tcl vl-controls-42101.body vl-controls-42101.tsv
```

## Evidence and cleanup

The retained bundle is
[`evidence/r2286_20261007vllex`](evidence/r2286_20261007vllex/). Its per-file
inventory is [`SHA256SUMS`](evidence/r2286_20261007vllex/SHA256SUMS). Raw
`/var/log/ltm` was captured before creation through each traffic phase and
cleanup in
[`ltm-continuous.log`](evidence/r2286_20261007vllex/appliance/ltm-continuous.log).

All owned VIPs, pools, rules, scripts, iApp/iCall objects and the disposable
data-group control were removed. Both post-cleanup object inventories contain
zero lines; the shared pre-existing `/Common/192.168.9.80` node remains. Both
owned backend processes and both owned tail processes were stopped and verified
absent. The evidence directories and reusable source harnesses remain by
design; no unrelated object or process was removed.

These conclusions are limited to the exact programs, events, byte producers,
four TMMs and BIG-IP build recorded above. Failed `lassign` and rebinding cases
are rejections, not evidence for their unreached receiver semantics.
