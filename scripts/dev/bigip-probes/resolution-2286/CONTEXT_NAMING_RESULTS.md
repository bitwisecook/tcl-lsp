# BIG-IP context-specific naming measurements

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
| VIP / backend | Primary `192.168.9.24:18111` / `192.168.9.80:18110`; counted-boundary run `192.168.9.24:18121` / `192.168.9.80:18120` |
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
`192.168.9.80:18110`, observed translated peer `192.168.9.24`, and returned all
responses through the VIP. The later counted-boundary baseline independently
proved the same translated peer at port 18120. The authoritative portable-index
rerun and every event comparison also reached all four units.

The exact version, virtual configuration, raw traffic, backend observations,
continuous LTM logs, and decoded counts are retained in the
[primary appliance archive](evidence/r2286_20261006naming/r2286p-appliance-evidence.tgz),
[primary dev archive](evidence/r2286_20261006naming/r2286p-dev-evidence.tgz),
and [decoded result index](evidence/r2286_20261006naming/decoded-results.json).

## Source and byte provenance

All payload and wrapper files were generated as ASCII with LF line endings and
contained no literal NUL or non-ASCII configuration-source byte. Runtime NUL
and non-ASCII values were reconstructed with `binary format H*`; `%c` results
were kept distinct from binary-produced bytes. Every archive was transferred
without text conversion and every file passed appliance-side `sha256sum -c`
before loading.

| Run | Purpose | Fixture archive SHA-256 | Manifest SHA-256 |
| --- | --- | --- | --- |
| `r2286p` | Complete primary matrix and lifecycle controls | `aa0dc6c5e5676d7ce0cab68476f7661a25da1c14de1bd3790d9432e147500592` | `0c7021b00ecceae4911f482e92c7e53071b6d531c02fbd60b2e58ed74e003268` |
| `r2286q` | Chunked exact event results and same-run HTTP controls | `d6082f475115dd934f7b6cbaac13028d7c943a11316de8dbcdb331a4c01a0dfd` | `b089b08b415927d9f44134edb14edb5e349e2492ceff0adb4c952cf72e233025` |
| `r2286r` | Corrected qualified scalar/array reuse control | `2df978fc390ed5ac466442a787732320159c16e005ea739cee11f6d074e9ce1f` | `829e33f4953529050f7ebe9852711d824527a6f09885bf46a462efc1cda1ed7f` |
| `r2286s` | Portable non-TMM array trace controls | `1705dfb475667046394dd2549e80abb07ceed1b3a6ee167de8d615127a2adc45` | `701004ce03235b412f508adfe77e6bfe5495b533b85bf20c2681959b07df351a` |
| `r2286t` | Counted-input, relocated/multiple-NUL array-root and index controls | `d99260e95dce1bac6834937430c4daa411f5a5a35406b78db423d5bf652eb076` | `d90eab3752e3582ef94d846148596b94024ff8ca7de423f02744e35abec23d18` |

The final generator SHA-256 is
`f6698b14f063f84affcd4d40c38a305d6844e28b9d2581a4ad73a589d5651dde`;
the decoder SHA-256 is
`8be0d5b865a310640a3f8c74f55da9b9297903c7917a288e49fa267938ffd2fd`.
The counted-boundary decoder SHA-256 is
`e96da76404745fa0541faab0ad8dc9e788f0d8a0d2541605fc6cf95787377981`.
The complete per-file wrapper, payload, hex-dump, size, and context hashes are
inside the retained manifests. The local evidence inventory is
[SHA256SUMS.local](evidence/r2286_20261006naming/SHA256SUMS.local).

## Result matrix

All rows below loaded successfully and returned outer catch status 0 unless a
rejection is stated. `tmsh`, `iApp`, and `iCall` mean an executed tmsh CLI
script, executed iApp implementation action, and delivered triggered-iCall
event respectively. Object creation alone was not counted as execution.

| Case | Payload SHA-256 | Reached contexts | Measured result |
| --- | --- | --- | --- |
| `nul_counted_last` | `e0b822bffae6a874bdd51c320ca72d446d6016219f9fe4d2258428bc16c458ea` | HTTP_REQUEST, tmsh, iApp, iCall | `A` and `A 00 42` names coexisted; `upvar` mutated only the counted name; unsetting the counted name preserved the plain name. Exact result bytes matched in all four contexts. |
| `nul_plain_last` | `42bbb29d837a8aa3c5051bb3ba5388c4fcccfe4cb645ad90e0ed4e5e83a4e96c` | HTTP_REQUEST, tmsh, iApp, iCall | Reverse write order did not alias either scalar name. Exact result bytes matched. |
| `nul_plain_unset_first` | `0953c937a7409680a7bcf30575f23c77aa3691052083c5726df7bc57c3482dd7` | HTTP_REQUEST, tmsh, iApp, iCall | Unsetting `A` left `A 00 42` and its linked value intact. Exact result bytes matched. |
| `nul_array_root` | `3f9483f0320860df7ebf1e53b170a3ecb702d00e4ed160066294baae86a92fd3` | HTTP_REQUEST; canonical tmsh/iApp/iCall reached a trace error | In TMM, roots ending `41` and `41 00 42` were distinct arrays. In non-TMM contexts the required trace callback called unavailable `TMM::cmp_group`, so traced writes/reads failed; those failures are not array-identity results. |
| `nul_array_index` | `bcce048b7c48239695680c3b2dab7d15fd4f256bfcbd3a8d292c1f185b58e64f` | HTTP_REQUEST; canonical tmsh/iApp/iCall reached a trace error | In TMM, indexes `41` and `41 00 42` were distinct elements. The canonical non-TMM boundary is the same trace-command failure. |
| `nul_array_root_portable` | `8fa234686f7534a4961585b9138d994f1d2016a8a81f6c456bb1f0abd8bc1654` | HTTP_REQUEST, tmsh, iApp, iCall | Separately hashed callback-only repair. All contexts returned result SHA-256 `bdc8a515be08639361b95f5ca92bdfbb8647121baecab7e6739a85a269d29610`: roots stayed distinct through write, read, `upvar`, and unset. |
| `nul_array_index_portable` | `27444ad7e4c3ffa8e72c8655f4b562a53fc362982b2ae3b41bf35297fc6b014d` | HTTP_REQUEST, tmsh, iApp, iCall | All contexts returned result SHA-256 `8f17dec9408761ab1945644e8a2e0cb050360eb34a28b810141bba8fce5f225f`: indexes stayed distinct through write, read, `upvar`, and unset. |
| `unicode_format_forward` | `eb4bfb8ce812be154cb9892ac8db4b29d147584f70b5bd4199b34de47af025ec` | HTTP_REQUEST, tmsh, iApp, iCall | `%c 233` produced byte `e9`; `e` plus `%c 769` produced `65 01`. Both names remained distinct. TMM lacked `encoding`; non-TMM `encoding convertto utf-8` succeeded. |
| `unicode_format_reverse` | `0cb94a67bbb36bf90514f916426085c99bfc87d68cc1e791b532843ac59b2cb2` | HTTP_REQUEST, tmsh, iApp, iCall | Reverse write/unset order preserved the same two cells and the same context-specific `encoding` boundary. |
| `unicode_bytes_forward` | `628ab017aa64f1daad41f08ebcf68b8b4489866376911b0b2e92b293f266a1a9` | HTTP_REQUEST, tmsh, iApp, iCall | Binary suffixes `c3 a9` and `65 cc 81` were distinct variable names. |
| `unicode_bytes_reverse` | `4635c663d565e650093ea2e2700bbf5d113505d9f91c54666300e15a045a2bb4` | HTTP_REQUEST, tmsh, iApp, iCall | Reverse write/unset order did not change byte identity. |
| `unicode_cross_producer` | `038f507380651e55a7fcd3357518af08666e4df03d3a9db9f3272c3f6dcdf73e` | HTTP_REQUEST, tmsh, iApp, iCall | `e9`, `c3a9`, `6501`, and `65cc81` were four independent names. Mutation and unset never crossed producer boundaries. Exact result bytes matched all contexts. |
| `commands_format` | `c081aec8ce405a772f28ddd0dceb43df46fcbe8d4807c906c9414721b4d0ae4e` | HTTP_REQUEST, tmsh, iApp, iCall | Dynamic namespace/proc creation, lookup, call, rename, new-name call, and cleanup succeeded for `e9` and `6501`. Old lookup became empty after rename. |
| `commands_bytes` | `1d50a4ee970a1e63516638e7b5357ed560b79fca2172656703619c369c774426` | HTTP_REQUEST, tmsh, iApp, iCall | The same operations succeeded independently for `c3a9` and `65cc81`. |
| `commands_cross_producer` | `730e9e5f8bc1870c8c486f12cb70abd8cd9b80b91603466b36906e8beed33af3` | HTTP_REQUEST, tmsh, iApp, iCall | Format- and binary-produced namespaces/procs coexisted and returned different values; renaming the binary-produced proc did not affect the format-produced proc. Exact result bytes matched all contexts. |
| `lexical_format_unbraced` | `5671de6fbceeeb8ce9cdb9f3e71d231e017a139fe5d13b9c896e4c40a3d9a054` | HTTP_REQUEST, tmsh, iApp, iCall | Context divergence: TMM stopped before `e9`; non-TMM consumed `e9` as part of the unbraced variable token. Both stopped before `01`. |
| `lexical_format_braced` | `7e18ff40d18e12b6ef7b3eafa9c4a908f17fc99baabecc401e4d82f5e15f074b` | HTTP_REQUEST, tmsh, iApp, iCall | Braced substitution consumed the complete raw name in every context. Exact result bytes matched. |
| `lexical_bytes_unbraced` | `dad5e4137cc43b66963ed3de844df11f15449dd9f987c07f214ce3730a5c81ba` | HTTP_REQUEST, tmsh, iApp, iCall | Context divergence: TMM stopped before `c3`/`cc`; non-TMM consumed the first byte (`c3` or `cc`) and stopped before the following byte. |
| `lexical_bytes_braced` | `9276abe9114ec4e88a73316f69156453a0f7f1641962bd1e586aefe6d96c7ed3` | HTTP_REQUEST, tmsh, iApp, iCall | Braced substitution consumed all `c3a9`/`65cc81` bytes in every context. Exact result bytes matched. |
| `lexical_clean` | `dca3e393ff7a0ef0c79bf089f66f82af1cc7974c83dbed957a6e4ee60f944b03` | HTTP_REQUEST, tmsh, iApp, iCall | Clean discriminator confirmed the lexical differences below without a second variable-name lookup. |
| `qualified_nul` corrected control | `cc5dbe00fd442bf974984a9280652a3b394404693d2ed05141586bcf932043f5` | HTTP_REQUEST, tmsh, iApp, iCall | Qualified scalar roots, array roots, and indexes containing `00 42` remained distinct. Results differed only because `namespace current` was `::script` in tmsh and `::` in the other three contexts. |
| `environment` | `4104a95c3dfaa82b6316d46718ef5bf67296b60e9e83e259738e2a3c8343f692` | HTTP_REQUEST, tmsh, iApp, iCall | Every context reported Tcl 8.4.6 / package Tcl 8.4. TMM lacked `encoding` and exposed the iRule command set; non-TMM contexts exposed Tcl/file/process commands instead. |
| `counted_input_boundaries` | `f1925eaf85382d68115f842ddccf108249de9f696efde04c66fbbaa31ea74a26` | HTTP_REQUEST | The request's exact source loaded and ran 32 times on every TMM. Short, long and relocated scalar prefixes remained distinct from the corresponding names containing two NULs. The one-line LTM result was truncated, as expected. |
| `counted_input_capture` | `1276bb8c7b70846a0362ff4ef31f7d3c0de6cd0a2719b98e143f2bbf8473afa8` | HTTP_REQUEST | Separately hashed capture-only repair returned the complete `rows` bytes. All 128 bodies were identical; decoded result SHA-256 `2b23a649a4650089e07d22c0d9e9982054e1e4a22728e719f754f93e0e93ba90`. |
| `array_root_boundaries` | `c89668f9995f180c6ddf853b8401be186f662938e52438aecfd1207136430585` | HTTP_REQUEST | Empty, short, long, relocated, colon-bearing and qualified roots, including leading/middle/two-NUL variants, retained exact bytes through both write/unset orders, reads, `upvar` and traces. A root spelling containing parentheses was syntax-bearing and is a counterexample, not root-identity evidence. |
| `array_index_boundaries` | `b428f284ae19cce77e0c019dc5bca3b50e6f2cbf510238bcae3e0eb8746db01f` | HTTP_REQUEST | Empty and punctuation-bearing indexes plus leading/middle/two-NUL variants retained exact bytes through both orders, reads, `upvar`, unset and trace callbacks. Decoded result SHA-256 `4a38c357ffbe63b8e53e1fdb1338fdd621725fdd81855ce3c54633970240d0b4`. |

The complete result hex, result SHA-256, Latin-1 lossless list projection,
context equality groups, loader output, trace tuples, and raw response-body hex
are in [decoded-results.json](evidence/r2286_20261006naming/decoded-results.json).

## TMM event comparison

Run `r2286q` used an identical payload for HTTP_REQUEST, CLIENT_ACCEPTED, and
RULE_INIT. The separately hashed event wrappers caught identity lookup and
emitted 400-hex-character chunks so the complete result survived the
1,024-byte LTM log-line limit. Each chunk set was complete and internally
consistent.

| Case | Payload SHA-256 | CLIENT_ACCEPTED wrapper SHA-256 | RULE_INIT wrapper SHA-256 | Exact result SHA-256 |
| --- | --- | --- | --- | --- |
| `nul_counted_last` | `c6d687f237c239600f5bd1cf76e16d371f584eea315f1338fceecb14149a64e9` | `0fe823726a6ccf0840813af84b19a2d9429c1ed44594be07e116de3b3c92f531` | `4ff8e64ebd4aebc3c0d7e5b5a9d691e4c2940c2d67129d618f12639a33cd4b57` | `85e71e85003bea2934299cf3e46e90a2b4e57e20c9549c8855cc37bc8ffb7791` |
| `nul_array_root` | `1a942ef715ac1b9655a3c34098864fa1f87a9feeb129d9afd95dba545de75fd0` | `8d898a8af4855ddb7ee2a8ffb7e9cb2625c441880bc826eeaee662dead58bf73` | `091175fdd12a532453f5798d007bc8f86f57f8f8958765222c5578eb817f1db3` | `9fd64024074015b88b07c7f74f57a5fbeec34a68a10fc2e4bfb9cf688539d010` |
| `nul_array_index` | `8e0c4f79d9213ef60f1b8725af64b25ef1cad809a06aa99643290dd04a1d7cf6` | `d2fb9f041c1f3ca4522a9f1e49ab5b5bce73ed46fce8ba3dbea218e55b104677` | `2f394939762fd3ad24eb534b751b5fd6b65c7e636fa19cf9076f9e5b72692007` | `d211a7cd9096ddee008b44997a4fe46c54a2255ec0f0e1171e513524e6ca548a` |
| `lexical_clean` | `7a55589049588f9bb5ddbe36b944c41b2294374942c348a969dfa56e65db543f` | `acd357e9b65459331a2c72a47b8300e5104975ade9e38ff317e7ca97e3dd8180` | `2cfed0047b594547dfee3e5dddb253559a4f50d1bfb2e4c2fdb031bf22a51090` | `2508dc8a3f13c4d8401bb3d7564de443dd64cf4233354a96e37f0fbd007d7128` |

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
forwarded 128 requests to `192.168.9.80`, where the server observed SNAT peer
`192.168.9.24` 128 times. Response headers proved 32 baseline connections on
each actual TMM. The exact counted-input source from the request matched the
Markdown code block byte-for-byte: 1,791 ASCII/LF bytes, no CR or NUL, SHA-256
`f1925eaf85382d68115f842ddccf108249de9f696efde04c66fbbaa31ea74a26`.
Its complete rule wrapper SHA-256 was
`35e634c49b43015e589dc4ab441a1e178319ceb58ef7d7b9977330c46b154033`.

The exact rule loaded with dynamic-name warnings and returned `OK` on all 128
requests. The first 128 consecutive exact-rule markers in the continuous LTM
capture contained 32 observations from each of TMM units 0, 1, 2 and 3. Its
single log record was longer than the appliance's retained line, so it cannot
be the source of complete row bytes. The separately hashed capture form changed
only the response step: it hex-encoded the already-computed `rows` value into
the HTTP body and added the TMM header. Its source/wrapper SHA-256 values were
`1276bb8c7b70846a0362ff4ef31f7d3c0de6cd0a2719b98e143f2bbf8473afa8`
and `361c2c5644f48cda56f328e974026aa4efa4f4d086a6f7d98e99a2ae7b8bc2e9`.
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
from callback spelling. All 128 response bodies had outer catch 0 and decoded
result SHA-256
`c79a936c910e9d3f82dab00670f3a0eefe8f939410d994f2f2374b8eb6be3885`.

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
rows, exact error bytes, response hashes, trace tuples and per-TMM equality are
in the [counted-boundary decoded results](evidence/r2286_20261006naming/r2286t-decoded-results.json).

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
runtime result. A separately hashed documented schedule omitted
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
  --vip 192.168.9.24 --vip-port 18111 \
  --backend 192.168.9.80 --backend-port 18110 \
  --source-commit 834329e46013c24f8c38b23e740524049c2a1e68

(cd /tmp/r2286x-fixtures && sha256sum -c SHA256SUMS)

# Start the raw LTM capture before creating any rule, partition, pool, or VIP.
tail -n 0 -F /var/log/ltm > /var/tmp/r2286x-ltm-raw.log &

# On BIG-IP, after transferring and rechecking the fixture tree:
tmsh load sys config merge file /var/tmp/r2286x-fixtures/base/backend.conf
tmsh create auth partition R2286_r2286x default-route-domain 0
tmsh load sys config merge file /var/tmp/r2286x-fixtures/base/lab.conf
tmsh load sys config merge file \
  /var/tmp/r2286x-fixtures/additional/lexical_clean.conf

# On dev.bragi0.com, with backend.py listening at 192.168.9.80:18110:
python3 "$PROBES/followup-traffic.py" \
  --vip 192.168.9.24 --port 18111 --run r2286x \
  --phase lexical-clean --path /lexical-clean \
  --source-ip 192.168.9.80 --source-port-start 47000 \
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
`summarize-context-naming-evidence.py` verifies raw response hashes,
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

- [Decoded byte-preserving result index](evidence/r2286_20261006naming/decoded-results.json)
- [Primary appliance evidence](evidence/r2286_20261006naming/r2286p-appliance-evidence.tgz)
- [Primary raw clients/backend](evidence/r2286_20261006naming/r2286p-dev-evidence.tgz)
- [Primary exact fixtures](evidence/r2286_20261006naming/r2286p-fixtures-v2.tgz)
- [Chunked event appliance evidence](evidence/r2286_20261006naming/r2286q-appliance-evidence.tgz)
- [Same-run event HTTP evidence](evidence/r2286_20261006naming/r2286q-http-dev-evidence.tgz)
- [Corrected qualified-name evidence](evidence/r2286_20261006naming/r2286r-appliance-evidence.tgz)
- [Portable array evidence](evidence/r2286_20261006naming/r2286s-appliance-evidence.tgz)
- [Isolated portable-root rerun](evidence/r2286_20261006naming/r2286s-root-dev-evidence.tgz)
- [Counted-boundary appliance evidence](evidence/r2286_20261006naming/r2286t-appliance-evidence.tgz)
- [Counted-boundary raw traffic/backend evidence](evidence/r2286_20261006naming/r2286t-dev-evidence.tgz)
- [Counted-boundary exact fixtures](evidence/r2286_20261006naming/r2286t-fixtures.tgz)
- [Counted-boundary decoded results](evidence/r2286_20261006naming/r2286t-decoded-results.json)
- [Complete local SHA-256 inventory](evidence/r2286_20261006naming/SHA256SUMS.local)

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
