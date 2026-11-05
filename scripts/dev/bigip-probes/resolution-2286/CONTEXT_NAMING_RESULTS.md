# BIG-IP context-specific naming measurements

> This historical summary is development material. Recovered scripts, questions, assumptions and results are recorded in the [per-script Tcl evidence](../../../../evidence/tcl/).

These are measured results from the active standalone BIG-IP lab appliance.
TMM iRules, tmsh CLI scripts, iApp implementation Tcl, iApp presentation/APL,
and iCall are reported separately. Standalone `tclsh` was not used as a
substitute for any BIG-IP execution context.

## Appliance, topology, and reached coverage

| Property | Measured value |
| --- | --- |
| Product | BIG-IP 21.1.0.1 |
| Build | 0.0.26 |
| Edition / hotfix | Point Release 1; no separate hotfix shown |
| Build date | Tue Jul 14 05:03:24 PDT 2026 |
| FIPS | Cryptographic Module for BIG-IP |
| Platform | Virtual Edition, system type Z100 |
| Topology | Standalone, active device |
| TMM process | PID 11604, `tmm.0 -T 4 --tmid 0 --npus 4 --platform Z100` |
| Actual TMM roster | `0:0`, `0:1`, `0:2`, `0:3` |
| VIP / backend | Primary `192.0.2.10:18111` / `192.0.2.201:18110`; counted-boundary run `192.0.2.10:18121` / `192.0.2.201:18120` |
| VIP profiles | TCP and HTTP |
| CMP | `cmp-enabled yes`, all CPUs |
| Source translation | SNAT `automap` |
| Tested source commit | `834329e46013c24f8c38b23e740524049c2a1e68` on `work` for the counted-input additions; the earlier context matrix was generated at `bedba02318f39c8e0d183165028db939f60cc8ca` |

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

`tmsh show sys tmm-info` enumerated `0.0`, `0.1`, `0.2`, and `0.3`, all in
PID 11604. Each authoritative 128-connection TMM traffic phase observed 32
executions per identity. Response headers supplied that identity for the
ordinary phases; the request's exact counted-input rule deliberately returned
no identity header, so its own 128 direct event markers supplied 32 observations
per unit. This is direct response/event evidence, not an inference from CMP
configuration. The primary backend baseline received 128 requests at
`192.0.2.201:18110`, observed translated peer `192.0.2.10`, and returned all
responses through the VIP. The later counted-boundary baseline independently
proved the same translated peer at port 18120. The authoritative portable-index
rerun and every event comparison also reached all four units.

The exact version, virtual configuration, raw traffic, backend observations,
continuous LTM logs, and decoded counts are retained in the
primary appliance archive,
primary dev archive,
and decoded result index.

## Source and byte provenance

The retained payload and wrapper sources use ASCII and LF. Runtime NUL and
non-ASCII values were constructed at runtime. The linked per-script evidence
shows the exact payload and records the scope of each captured result.

## Result matrix

All rows below loaded successfully and returned outer catch status 0 unless a
rejection is stated. `tmsh`, `iApp`, and `iCall` mean an executed tmsh CLI
script, executed iApp implementation action, and delivered triggered-iCall
event respectively. Object creation alone was not counted as execution.

| Case | Reached contexts | Measured result |
| --- | --- | --- |
| [`nul_counted_last`](../../../../evidence/tcl/bigip-r2286p-nul-counted-last.md) | HTTP_REQUEST, tmsh, iApp, iCall | `A` and `A 00 42` names coexisted; `upvar` mutated only the counted name; unsetting the counted name preserved the plain name. Exact result bytes matched in all four contexts. |
| [`nul_plain_last`](../../../../evidence/tcl/bigip-r2286p-nul-plain-last.md) | HTTP_REQUEST, tmsh, iApp, iCall | Reverse write order did not alias either scalar name. Exact result bytes matched. |
| [`nul_plain_unset_first`](../../../../evidence/tcl/bigip-r2286p-nul-plain-unset-first.md) | HTTP_REQUEST, tmsh, iApp, iCall | Unsetting `A` left `A 00 42` and its linked value intact. Exact result bytes matched. |
| [`nul_array_root`](../../../../evidence/tcl/bigip-r2286p-nul-array-root.md) | HTTP_REQUEST; canonical tmsh/iApp/iCall reached a trace error | In TMM, roots ending `41` and `41 00 42` were distinct arrays. In non-TMM contexts the required trace callback called unavailable `TMM::cmp_group`, so traced writes/reads failed; those failures are not array-identity results. |
| [`nul_array_index`](../../../../evidence/tcl/bigip-r2286p-nul-array-index.md) | HTTP_REQUEST; canonical tmsh/iApp/iCall reached a trace error | In TMM, indexes `41` and `41 00 42` were distinct elements. The canonical non-TMM boundary is the same trace-command failure. |
| [`nul_array_root_portable`](../../../../evidence/tcl/bigip-r2286s-nul-array-root-portable.md) | HTTP_REQUEST, tmsh, iApp, iCall | In the isolated root rerun and the non-TMM contexts, roots stayed distinct through write, read, `upvar`, and unset. |
| [`nul_array_index_portable`](../../../../evidence/tcl/bigip-r2286s-nul-array-index-portable.md) | HTTP_REQUEST, tmsh, iApp, iCall | In the recorded index runs, indexes stayed distinct through write, read, `upvar`, and unset. |
| [`unicode_format_forward`](../../../../evidence/tcl/bigip-r2286p-unicode-format-forward.md) | HTTP_REQUEST, tmsh, iApp, iCall | `%c 233` produced byte `e9`; `e` plus `%c 769` produced `65 01`. Both names remained distinct. TMM lacked `encoding`; non-TMM `encoding convertto utf-8` succeeded. |
| [`unicode_format_reverse`](../../../../evidence/tcl/bigip-r2286p-unicode-format-reverse.md) | HTTP_REQUEST, tmsh, iApp, iCall | Reverse write/unset order preserved the same two cells and the same context-specific `encoding` boundary. |
| [`unicode_bytes_forward`](../../../../evidence/tcl/bigip-r2286p-unicode-bytes-forward.md) | HTTP_REQUEST, tmsh, iApp, iCall | Binary suffixes `c3 a9` and `65 cc 81` were distinct variable names. |
| [`unicode_bytes_reverse`](../../../../evidence/tcl/bigip-r2286p-unicode-bytes-reverse.md) | HTTP_REQUEST, tmsh, iApp, iCall | Reverse write/unset order did not change byte identity. |
| [`unicode_cross_producer`](../../../../evidence/tcl/bigip-r2286p-unicode-cross-producer.md) | HTTP_REQUEST, tmsh, iApp, iCall | `e9`, `c3a9`, `6501`, and `65cc81` were four independent names. Mutation and unset never crossed producer boundaries. Exact result bytes matched all contexts. |
| [`commands_format`](../../../../evidence/tcl/bigip-r2286p-commands-format.md) | HTTP_REQUEST, tmsh, iApp, iCall | Dynamic namespace/proc creation, lookup, call, rename, new-name call, and cleanup succeeded for `e9` and `6501`. Old lookup became empty after rename. |
| [`commands_bytes`](../../../../evidence/tcl/bigip-r2286p-commands-bytes.md) | HTTP_REQUEST, tmsh, iApp, iCall | The same operations succeeded independently for `c3a9` and `65cc81`. |
| [`commands_cross_producer`](../../../../evidence/tcl/bigip-r2286p-commands-cross-producer.md) | HTTP_REQUEST, tmsh, iApp, iCall | Format- and binary-produced namespaces/procs coexisted and returned different values; renaming the binary-produced proc did not affect the format-produced proc. Exact result bytes matched all contexts. |
| [`lexical_format_unbraced`](../../../../evidence/tcl/bigip-r2286p-lexical-format-unbraced.md) | HTTP_REQUEST, tmsh, iApp, iCall | Context divergence: TMM stopped before `e9`; non-TMM consumed `e9` as part of the unbraced variable token. Both stopped before `01`. |
| [`lexical_format_braced`](../../../../evidence/tcl/bigip-r2286p-lexical-format-braced.md) | HTTP_REQUEST, tmsh, iApp, iCall | Braced substitution consumed the complete raw name in every context. Exact result bytes matched. |
| [`lexical_bytes_unbraced`](../../../../evidence/tcl/bigip-r2286p-lexical-bytes-unbraced.md) | HTTP_REQUEST, tmsh, iApp, iCall | Context divergence: TMM stopped before `c3`/`cc`; non-TMM consumed the first byte (`c3` or `cc`) and stopped before the following byte. |
| [`lexical_bytes_braced`](../../../../evidence/tcl/bigip-r2286p-lexical-bytes-braced.md) | HTTP_REQUEST, tmsh, iApp, iCall | Braced substitution consumed all `c3a9`/`65cc81` bytes in every context. Exact result bytes matched. |
| [`lexical_clean`](../../../../evidence/tcl/bigip-r2286p-lexical-clean.md) | HTTP_REQUEST, tmsh, iApp, iCall | Clean discriminator confirmed the lexical differences below without a second variable-name lookup. |
| [`qualified_nul`](../../../../evidence/tcl/bigip-r2286r-qualified-nul.md) corrected control | HTTP_REQUEST, tmsh, iApp, iCall | Qualified scalar roots, array roots, and indexes containing `00 42` remained distinct. Results differed only because `namespace current` was `::script` in tmsh and `::` in the other three contexts. |
| [`environment`](../../../../evidence/tcl/bigip-r2286p-environment.md) | HTTP_REQUEST, tmsh, iApp, iCall | Every context reported Tcl 8.4.6 / package Tcl 8.4. TMM lacked `encoding` and exposed the iRule command set; non-TMM contexts exposed Tcl/file/process commands instead. |
| [`counted_input_boundaries`](../../../../evidence/tcl/bigip-r2286t-counted-input-boundaries.md) | HTTP_REQUEST | The request's exact source loaded and ran 32 times on every TMM. Short, long and relocated scalar prefixes remained distinct from the corresponding names containing two NULs. The one-line LTM result was truncated, as expected. |
| [`counted_input_capture`](../../../../evidence/tcl/bigip-r2286t-counted-input-capture.md) | HTTP_REQUEST | The capture-only repair returned the complete `rows` bytes; all 128 bodies were identical. |
| [`array_root_boundaries`](../../../../evidence/tcl/bigip-r2286t-array-root-boundaries.md) | HTTP_REQUEST | Empty, short, long, relocated, colon-bearing and qualified roots, including leading/middle/two-NUL variants, retained exact bytes through both write/unset orders, reads, `upvar` and traces. A root spelling containing parentheses was syntax-bearing and is a counterexample, not root-identity evidence. |
| [`array_index_boundaries`](../../../../evidence/tcl/bigip-r2286t-array-index-boundaries.md) | HTTP_REQUEST | Empty and punctuation-bearing indexes plus leading/middle/two-NUL variants retained exact bytes through both orders, reads, `upvar`, unset and trace callbacks. |

The complete result hex, Latin-1 lossless list projection,
context equality groups, loader output, trace tuples, and raw response-body hex
are in `decoded-results.json`.

## TMM event comparison

Run `r2286q` used an identical payload for HTTP_REQUEST, CLIENT_ACCEPTED, and
RULE_INIT. The event wrappers caught identity lookup and
emitted 400-hex-character chunks so the complete result survived the
1,024-byte LTM log-line limit. Each chunk set was complete and internally
consistent.

| Case |
| --- |
| [`nul_counted_last`](../../../../evidence/tcl/bigip-r2286q-nul-counted-last.md) |
| [`nul_array_root`](../../../../evidence/tcl/bigip-r2286q-nul-array-root.md) |
| [`nul_array_index`](../../../../evidence/tcl/bigip-r2286q-nul-array-index.md) |
| [`lexical_clean`](../../../../evidence/tcl/bigip-r2286q-lexical-clean.md) |

For every row, HTTP_REQUEST, 128 CLIENT_ACCEPTED executions, and four
RULE_INIT executions returned byte-identical results. CLIENT_ACCEPTED reached
each TMM 32 times. RULE_INIT emitted once from each of `0:0`–`0:3`; this is
direct observed identity coverage, not a conclusion from execution count.

## Lexical-name boundary

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

## Variables, arrays, aliases, and traces

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

## Counted-key input boundaries

Run `r2286t` used a fresh VIP on port 18121 and backend port 18120. Its baseline
forwarded 128 requests to `192.0.2.201`, where the server observed SNAT peer
`192.0.2.10` 128 times. Response headers proved 32 baseline connections on
each actual TMM. The exact counted-input source from the request matched the
Markdown code block byte-for-byte: 1,791 ASCII/LF bytes, no CR or NUL.

The exact rule loaded with dynamic-name warnings and returned `OK` on all 128
requests. The first 128 consecutive exact-rule markers in the continuous LTM
capture contained 32 observations from each of TMM units 0, 1, 2 and 3. Its
single log record was longer than the appliance's retained line, so it cannot
be the source of complete row bytes. The separate capture form changed
only the response step: it hex-encoded the already-computed `rows` value into
the HTTP body and added the TMM header.
All 128 full results were byte-identical across the four TMMs.

| Input domain | Exact measured HTTP_REQUEST result |
| --- | --- |
| Scalar prefixes `x`, 59-byte long prefix, and `relocated_2286_key` | Plain and `plain 00 42 00 43` names both wrote/read with catch 0 as `PLAIN` and `COUNTED`; unsetting the counted name left `PLAIN`. |
| Empty dynamic variable input | Write/read catch 0, value `GRAMMAR_VALUE`. |
| `()`, `(k)`, `A()`, `A(k)`, `A(k(l))` | Write/read catch 0. These are accepted combined variable forms; the result alone does not make parentheses part of an opaque scalar key. |
| `A(k)tail`, `:`, `A:B` | Write/read catch 0. |
| `A::B` with no namespace `A` | Write rejected: `can't set "A::B": parent namespace doesn't exist`; read rejected: `can't read "A::B": no such variable`. This is qualified lookup, not a generic colon rejection. |
| Array indexes ``, `()`, `(k)`, `A()`, `A(k)`, `A(k(l))`, `A(k)tail`, `:`, `A:B`, `A::B` | Every index wrote, read and unset with catch 0; trace `name2` contained the exact input bytes. Colons and parentheses inside an already-separated index are ordinary index bytes. |

The array-root control used unrelated short, long and relocated prefixes,
leading NUL, middle NUL, two NULs, an empty root, a colon-bearing root, and a
qualified root under a created ASCII namespace. Except for the parentheses
case below, every write/read/upvar succeeded in both orders. Unsetting either
plain or counted root preserved the other value. Direct trace callbacks
received the complete root bytes and index `6b`; the alias write reported root
hex `726f6f745f616c696173` (`root_alias`), keeping selected identity separate
from callback spelling. All 128 response bodies had outer catch 0 and identical decoded results.

The parentheses-root row is an important counterexample. The control appended
`(k)` to root spelling `root(k)`. The resulting `root(k)(k)` was parsed as a
combined variable form rather than an opaque root plus a separately supplied
index. The trace attached to `root(k)` did not observe the writes/reads, and in
the counted-first order `unset root(k)` returned catch 1 with exact result:

```text
can't unset "root(k)": no such element in array
```

The successful value reads in that row therefore address differently parsed
array elements and do not establish an independent array root named `root(k)`.
A compiler interface must separate root and index before rendering Tcl combined
variable syntax; concatenating `root + "(" + index + ")"` is not lossless for
arbitrary root spellings.

The array-index control independently supplied indexes to one ASCII root. All
grammar indexes above, empty/leading-NUL indexes, unrelated short/long prefixes,
middle NUL, and two-NUL values succeeded under reversed writes and unsets.
Plain and counted cells retained `PLAIN`/`COUNTED`, `upvar` selected only the
counted cell, and the survivor discriminator was correct in both orders. Trace
`name1` was the fixed root and `name2` preserved every exact index byte. All 128
responses were identical across the four TMMs with outer catch 0.

These added input-boundary controls reached TMM HTTP_REQUEST only. They do not
transfer the root/index grammar or parentheses counterexample to tmsh, iApp or
iCall.

The exact and capture rules both produced the following loader warnings at the
same source lines; the loader nevertheless accepted each rule:

```text
line 13: [variable reference used where variable name expected][$plain]
line 14: [variable reference used where variable name expected][$name]
line 15: [variable reference used where variable name expected][$plain]
line 16: [variable reference used where variable name expected][$name]
line 19: [variable reference used where variable name expected][$input]
line 23: [variable reference used where variable name expected][$name]
line 24: [variable reference used where variable name expected][$plain]
line 27: [variable reference used where variable name expected][$plain]
line 32: [variable reference used where variable name expected][$input]
line 35: [variable reference used where variable name expected][$input]
line 38: [variable reference used where variable name expected][$input]
```

The array-root and array-index wrappers loaded without warnings. Complete raw
rows, exact error bytes, trace tuples and per-TMM equality are
in the counted-boundary decoded results.

## Namespaces, procedures, rename, and command lookup

The command controls created absolute namespaces and procedures using dynamic
names, called them, resolved them with `namespace which -command`, renamed
them, proved the old spelling no longer resolved, called the renamed spelling,
and deleted the namespace. All operations succeeded in all four contexts.

No normalization was observed. Format-produced `e9` and `6501` names remained
distinct from binary-produced `c3a9` and `65cc81` names even when they render
similarly. The cross-producer procedures returned independent values before
and after rename. These results establish command identity for the tested
non-NUL names only; command-name NUL remains untested.

`namespace current` was `::script` in the tmsh CLI wrapper and `::` in TMM,
iApp implementation, and iCall. `namespace which -command set` returned
`::set` everywhere. This is an observed execution-namespace difference and
must not be inferred from the common Tcl patchlevel.

## iCall and iApp boundaries

Triggered iCall executed all payloads and produced its own scriptd marker.
The original periodic configuration was rejected literally:

```text
Syntax Error:(/var/tmp/r2286p-fixtures-v2/icall-lifecycles/periodic.conf at line: 27) invalid character following 'now'
```

The rejected spelling was `first-occurrence now+2s`; it was not treated as a
runtime result. A separate documented schedule omitted
`first-occurrence`, used `interval 5` and `last-occurrence now+1m`, loaded, and
executed three times (15 payload markers). It was then set inactive and
deleted. The perpetual handler loaded inactive, was explicitly started,
emitted its five representative markers once, was explicitly stopped, and was
deleted. No handler or script remained. The representative canonical array
payloads retain the non-TMM `TMM::cmp_group` trace failure; the portable array
results above are authoritative for non-TMM array identity.

The iApp implementation action actually executed for every payload. The APL
template and service containing the exact `choice ... tcl {}` callback loaded,
but service creation emitted zero callback markers. Live tmsh help exposed no
presentation renderer/execution command. No documented presentation execution
path was reached, so iApp presentation Tcl is **unreached**, not inferred from
iApp implementation Tcl.

## Literal loader versus runtime values

All successful naming controls transported ASCII hex in configuration source
and created NUL/non-ASCII names dynamically. They do not establish acceptance
of literal NUL, UTF-8, CRLF, continuation, split-line, brace, escape, emoji, or
grapheme bytes in iRule configuration source. Those literal loader questions
remain scoped to the earlier reports and their exact fixtures.

The only naming-suite loader rejection was the unsupported periodic time
spelling above. The APL boundary was accepted configuration with an unreached
callback, not a load rejection. Inner lexical and trace errors were dynamic
runtime results, not loader errors.

## Reproduction

```bash
PROBES=scripts/dev/bigip-probes/resolution-2286

python3 "$PROBES/generate-context-naming-controls.py" \
  --run r2286x --out /tmp/r2286x-fixtures \
  --vip 192.0.2.10 --vip-port 18111 \
  --backend 192.0.2.201 --backend-port 18110 \
  --source-commit 834329e46013c24f8c38b23e740524049c2a1e68

# Start the raw LTM capture before creating any rule, partition, pool, or VIP.
tail -n 0 -F /var/log/ltm > /var/tmp/r2286x-ltm-raw.log &

# On BIG-IP, after transferring and rechecking the fixture tree:
tmsh load sys config merge file /var/tmp/r2286x-fixtures/base/backend.conf
tmsh create auth partition R2286_r2286x default-route-domain 0
tmsh load sys config merge file /var/tmp/r2286x-fixtures/base/lab.conf
tmsh load sys config merge file \
  /var/tmp/r2286x-fixtures/additional/lexical_clean.conf

# On dev.bragi0.com, with backend.py listening at 192.0.2.201:18110:
python3 "$PROBES/followup-traffic.py" \
  --vip 192.0.2.10 --port 18111 --run r2286x \
  --phase lexical-clean --path /lexical-clean \
  --source-ip 192.0.2.201 --source-port-start 47000 \
  --requests 128 --expect-units 0:0,0:1,0:2,0:3 \
  --out /tmp/traffic-lexical-clean.jsonl

# Exact non-TMM wrappers are generated per payload.
bash "$PROBES/run-followup-contexts.sh" \
  r2286x18 /var/tmp/r2286x-fixtures/contexts/lexical_clean \
  /var/tmp/r2286x-context-evidence

# The prescribed counted-input rule and the capture-only repair are distinct.
tmsh load sys config merge file \
  /var/tmp/r2286x-fixtures/additional/counted_input_boundaries.conf
tmsh modify ltm virtual /R2286_r2286x/__tcl_lsp_2286_r2286x_vs \
  rules '{ /Common/__tcl_lsp_2286_r2286x_counted_input_boundaries }'

tmsh load sys config merge file \
  /var/tmp/r2286x-fixtures/additional/counted_input_capture.conf
tmsh modify ltm virtual /R2286_r2286x/__tcl_lsp_2286_r2286x_vs \
  rules '{ /Common/__tcl_lsp_2286_r2286x_counted_input_capture }'
```

The committed decoder command lines are fully specified by `--help`.
`summarize-context-naming-evidence.py` reads the retained response bytes,
reconstructs chunked event results, preserves embedded NUL in JSON, and emits
context equality groups. `summarize-counted-boundaries.py` verifies and decodes
the exact-source repair plus the root/index boundary traffic.

## Cleanup and retained evidence

All owned rules, virtual servers, pools, nodes, partitions, tmsh scripts, iApp
templates/services, iCall scripts/handlers, backend listeners, capture
processes, transferred fixtures, and remote evidence paths were removed by
exact name after local archive verification. Final `tmsh list` checks returned
not-found for each named object. Ports 18110/18111 and the later 18120/18121
pair had no owned listener or virtual remaining. The verified `r2286t` remote
fixture, evidence and archive paths were then removed. The required pre-existing
`/Common/self_1nic` was not removed. No incomplete cleanup or known test runtime
state remains.

The final appliance state still has `auto-check disabled`, `auto-phonehome
disabled`, and `f5_update_checker down, No action required`.

Retained evidence bundle:

- Decoded byte-preserving result index
- Primary appliance evidence
- `Primary raw clients/backend`
- Primary exact fixtures
- Chunked event appliance evidence
- Same-run event HTTP evidence
- Corrected qualified-name evidence
- Portable array evidence
- Isolated portable-root rerun
- Counted-boundary appliance evidence
- `Counted-boundary raw traffic/backend evidence`
- Counted-boundary exact fixtures
- Counted-boundary decoded results

Two non-authoritative traffic records are retained rather than hidden:
`r2286s` portable-index's first phase had two client-side `EADDRINUSE` bind
errors before a clean 128-request rerun, and its initially labelled portable
root phase had the index rule attached. The isolated root archive and clean
index rerun are the authoritative results cited above.

## Coverage limits

- The appliance has one TMM group with four units. Cross-blade, multi-group,
  HA-peer, and failover behavior are untested.
- NUL in a namespace component and NUL in a command/procedure name are
  untested. NUL was measured in scalar names, qualified variable tails, array
  roots, and array indexes.
- iApp presentation/APL Tcl was accepted but not executed; that mechanism is
  unreached.
- Suspended entered procedures and replacement during an entered frame are
  untested here.
- Persistence across configuration reload, TMM restart, or appliance reboot
  is untested.
- Literal-source byte acceptance was not repeated and must not be inferred
  from the dynamic runtime results.
- The added counted-input/root/index boundary controls ran only in TMM
  HTTP_REQUEST. Their behavior is not assigned to tmsh, iApp or iCall.
