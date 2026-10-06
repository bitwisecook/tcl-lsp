# BIG-IP name identity and callable lifetime follow-up

These are measured results from the active standalone BIG-IP lab appliance. TMM iRule, tmsh CLI script, iApp implementation and triggered iCall results are identified separately. Standalone `tclsh` was not used as a substitute for any BIG-IP execution context.

## Appliance, topology and coverage

| Property | Measured value |
| --- | --- |
| Product | BIG-IP 21.1.0.1 |
| Build | 0.0.26 |
| Edition / hotfix | Point Release 1; no separate hotfix shown |
| Build date | Tue Jul 14 05:03:24 PDT 2026 |
| Platform | BIG-IP Virtual Edition, system type Z100, four physical CPU cores |
| Topology | Standalone, active device |
| TMM process | PID 28022, `tmm.0 -T 4 --tmid 0 --npus 4 --platform Z100` |
| Actual reached TMM roster | `0:0`, `0:1`, `0:2`, `0:3` |
| VIP / backend | `192.168.9.24:18101` / `192.168.9.80:18100` |
| VIP profiles | TCP and HTTP |
| CMP | `cmp-enabled yes` |
| Source translation | `automap` |
| Tested source commit | `8ee8ad7c0df9eaab539f9663869ef664f20853e0` on branch `work` |

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

The complete version, failover, TMM process and redacted platform evidence is retained in [appliance evidence](evidence/r2286_20261006m/appliance/raw/version.txt), [failover evidence](evidence/r2286_20261006m/appliance/raw/failover.txt), [TMM process evidence](evidence/r2286_20261006m/appliance/raw/tmm-processes.txt) and [platform summary](evidence/r2286_20261006m/appliance/platform-summary.txt).

Every one of the 59 HTTP traffic phases completed without a transport error. Every non-backend phase returned one byte-identical response body across all requests and reached each actual TMM pair exactly 32 times in 128 fresh connections. This is direct response-header coverage, not an inference from CMP configuration or request count. The machine-readable counts and decoded response bytes are in [decoded-results.json](evidence/r2286_20261006m/decoded-results.json).

The backend received all 128 initial requests at `192.168.9.80:18100` with peer address `192.168.9.24`, the BIG-IP self address. The client connected to the VIP at `192.168.9.24:18101` and received the backend response through that VIP. This proves the tested path used SNAT automap and had a working return path. See [backend observations](evidence/r2286_20261006m/dev/backend.log) and [raw client responses](evidence/r2286_20261006m/dev/traffic/backend-initial.jsonl).

## Source and byte identity

Run `r2286m` used ASCII, LF-only, NUL-free payload files. Each iRule wrapper transported the exact payload bytes as hexadecimal and dynamically evaluated the reconstructed value. This keeps runtime parsing distinct from literal configuration-source acceptance. The v5 bundle supplied the principal cases; v6 reproduced every pre-existing payload and wrapper byte-for-byte and added the separately named fresh-owner lifetime controls.

| Artifact | SHA-256 |
| --- | --- |
| v5 fixture archive | `485a6355c3c85468614ca2c91ceb06eb24671fef12d8518594fd35ee37e89ee7` |
| v5 manifest | `989bdffc88ade0265b04c88bf416cf43b7fbe7ce483f82e0f7d441dd9e882d90` |
| v6 fixture archive | `f71752d905651e9e7fb9e90c973c8f696a53cbb74502002dd2048b702c905c2e` |
| v6 manifest | `9417032204aab1cc72db49d136b6233c9cc172c3a576bdf027b1e4f7181997c7` |
| Cross-context archive | `e974eccf8a91434e575151421f8e8bf6312013cbf32bc8ec26363ed05b86b534` |
| Cross-context manifest | `83c52486f6adc741fd00462d3dab869ad1b12c1cb964255a5924dbe244172115` |
| Exact expression payload | `041daa50f939c68691a98d5a138d54f7256edf8ded0765d151e8077108af58bc` |
| Final appliance evidence archive | `39109634e97d6780c27263ac2eee8027db682e34415a439ad57f092baf436d88` |
| Continuous raw `/var/log/ltm` | `44bbb0669b671ae651de3ca1f191f2788a18805ad5526ce094f7816d43b54ca4` |
| Decoded result index | `4dc32dddeb5a60b758a03b04d0a9271e83bdf37c92662d9c7e7b4e519f1d89a9` |

The appliance recomputed every transferred file hash before loading it. The retained sources, manifests and archives are under [fixtures](evidence/r2286_20261006m/fixtures/); the appliance-side verification is [v5](evidence/r2286_20261006m/appliance/raw/appliance-file-verification-v5.txt), [v6](evidence/r2286_20261006m/appliance/raw/appliance-file-verification-v6.txt) and [cross-context](evidence/r2286_20261006m/appliance/raw/contexts/appliance-file-verification.txt).

## Name-identity result matrix

All listed wrappers loaded successfully and returned outer catch status 0 on all four TMMs. Inner catch statuses are stated explicitly.

| Case | Payload SHA-256 | Wrapper SHA-256 | Measured TMM result |
| --- | --- | --- | --- |
| `nul_counted_last` | `cba6f800a9c3e80c17cc08434200fbba2828c413460c91682138879562393df6` | `1d5a364e3c8e48e0d4e3825b3df00298aff1e6b4071a96a569c573a9e3a47b05` | Plain and `A 00 B` scalar names were distinct; `upvar`, read and unset addressed only the counted name |
| `nul_plain_last` | `0d946243ff3331524c0a55a41cc48462fbcdf5b81762025dc75fd499776ce50c` | `9921f01c557a7917e0e93975bc9daf5a84b3e40f6e1c3bd3b47ad80c20d5e80b` | Reversing write order did not alias or overwrite either name |
| `nul_plain_unset_first` | `fce2466c347362948b78530de8786a7d27111c739b300dac27e3240c312ca04a` | `421f200e8ff47ce5480ee96178c5c483d76bb3d7c54857978a21d1ac6f37ac96` | Unsetting the plain name left the counted name and its linked value intact |
| `nul_array_root` | `ac81e5dbd1ba443199d88d87221fe55f5db9f7474e1c42ef1f2ad174af551fdf` | `32931180ef11228bac2b8a8922000a83482c44f9994991cb81b375895061ffa7` | Plain and NUL-bearing array roots were distinct; root unset and `upvar` affected only the selected array |
| `nul_array_index` | `aeaac8f74458ac99a752be410304584703a8a13a071bd4f13fdedd82f4d3b070` | `8b01c6367d9d90bab44d43d1044d1311267d16cb025e7ce09e06ef7267a30b0a` | Index `41` and index `41 00 42` were distinct elements; element unset and `upvar` affected only the counted index |
| `unicode_format_forward` | `3aa03c3acdd0dc093f25f8868e1306b8fd519eb8cda7532a1df3db5362a0a2a3` | `99b72528d0e6eca1727cdff238443f2b67ef5fc3e6d916486580e298414ddc92` | `%c 233` produced raw suffix `e9`; `%c 769` produced `65 01`; names remained distinct |
| `unicode_format_reverse` | `813837e4613d89e36194dbd330d1b81d0b320706e30a73facd39147ec49ec3a0` | `35d00a28bd3e3c3d3d1735d391166cb3d50a5c43e00ecdf3c763621e80c84f1d` | Reverse write/unset order did not change identity |
| `unicode_bytes_forward` | `65f2f9c46f497a38602168109806652a7415040fbc67959c82fa002ead7ae4b7` | `57f57492859bbe116e493555a1b2fc6a677bb31c5db97612bb41d31cc6aa0a75` | Byte suffixes `c3 a9` and `65 cc 81` remained distinct |
| `unicode_bytes_reverse` | `0286dad4d8f16ee0963772c5bac720f2254539fe856c3d84e5d31a63c9df3a84` | `0de395804c938f4f97c1a3d6a1c75810e540303652a7f070652cb67e657a8e3b` | Reverse write/unset order did not change identity |
| `unicode_cross_producer` | `091fcff7300e9c3a1695a8dc575d922a25b2dbb2f459df085a0e42162d2e1bea` | `7713e3b1f8e68c19603e386b03a10a8d8e3012a58e1358ddc5c103f9aa1379e1` | All four raw spellings (`e9`, `c3a9`, `6501`, `65cc81`) were separate variable names; mutation and unset never crossed producers |
| `lexical_format_unbraced` | `c55f583a23fd2e0eae66654e8c54d1f0b0b176aecf44867e5b9809bd648379bf` | `8494917f99809892c986f78dd72cf08749c980604d1bb3a54dc8623fbecdb1ec` | Dynamic writes succeeded; evaluated unbraced tokens stopped before `e9` or `01` and failed while reading the shorter lexical name |
| `lexical_format_braced` | `6e43ed8c0e18f80195c9fd962d5e424e42a8ea21670dbfe77b650f03a6b9c803` | `e1ca6c5ccf4e89653457477da4e3f7cbb2d5c821fad28b1274d8faf6f4096787` | The full braced raw name resolved to its value; the surrounding `set ${name}` script then attempted to read a variable named `PRE_VALUE` or `DECOMPOSED_VALUE` and failed |
| `lexical_bytes_unbraced` | `fba2cd37d875209f86cb6f2d83cf315002445076379cfb34928d92fcc3184e35` | `9a4ad39c0a030674c09db5654412907b04d7578bbecfc9fb9c39e819e8881adc` | Unbraced token scanning stopped before `c3` or `cc`, producing a shorter-name read failure |
| `lexical_bytes_braced` | `611248007d36153d4c6e6559766bcdb9dae130971ee5f2a1e3b8d7089b775643` | `c6f12dede3aedd37b1c031c1c3ec50882948ee34205cb62f6ee8157a4be1919d` | The full `c3a9` and `65cc81` braced names resolved; the second `set` lookup failed on the returned value name |
| `commands_format` | `0486fe6b1bb3c748a9f23ba1f4e9d302590794d3a5cda179ea1ec184a3940b69` | `ef598faed0951bd6fe81f794b254481a84e9232d5a5f1b3df280d8fd3364ab61` | Dynamic namespace/proc creation, lookup, call and rename succeeded for `e9` and `6501` names; the old command disappeared after rename |
| `commands_bytes` | `4007963e00029f6263be93956b8f7baabf506039a3eec0072573e50c42436804` | `38e64e308931db4f6bff577a4df5cb3de2e448fdc1d50392e882c8d026d686f5` | The same operations succeeded independently for `c3a9` and `65cc81` names |
| `commands_cross_producer` | `99712a42104c7411ff853e405b1d057cacba271bd50e3ea571fe649834d664ed` | `beccb39495c06705244537d4f1f963967b683ea92f1e1a116db6e77e51bda51e` | Format- and byte-produced namespaces/procs coexisted and returned different values; renaming a byte-produced proc did not affect its format-produced counterpart |
| `expressions` | `041daa50f939c68691a98d5a138d54f7256edf8ded0765d151e8077108af58bc` | `df4e3532d0f52c3374e2ed99c7c96740ecc3e59c589037d352dcde9ca979b820` | Exact nine-row result is reported below |

### Embedded NUL and trace arguments

The NUL-bearing scalar had length 29 versus 27 for its plain control. The array-root variable references had lengths 38 versus 36, and the NUL-bearing index had length 3 versus 1. In every case the names could coexist, be read independently, be targeted by `upvar`, and be unset independently. This is a counterexample to C-string truncation or name canonicalisation at the embedded NUL.

Variable traces accepted dynamically and fired on every actual TMM. For the index case, callbacks received the ASCII root unchanged and index hex `41` or `410042`, including the NUL. For the root case, callbacks received the full root hex with or without `0042` and index `6b`; the `upvar` write callback reported root name hex `616c696173` (`alias`) while mutating the linked counted array. The raw log contains 3,200 `R2286TRACE` rows and preserves the operation letters and TMM identities in [ltm-raw.log](evidence/r2286_20261006m/appliance/raw/ltm-raw.log).

### Unicode producer identity and lexical resolution

The TMM runtime is byte-oriented for these controls. `format %c 233` returned one byte `e9`, not UTF-8 `c3a9`; `format %c 769` returned byte `01`, so the constructed suffix was `6501`, not UTF-8 combining acute `65cc81`. `binary format H*` preserved the supplied UTF-8 bytes. All four byte sequences remained distinct as variable, namespace and procedure names. No NFC/NFD equivalence was observed. The `encoding` command was unavailable in TMM and returned `invalid command name "encoding"`.

Unbraced `$name` parsing did not consume the non-ASCII/control bytes into the lexical variable token. Braced `${name}` parsing did consume the entire raw name and reached its stored value. The subsequent error naming `PRE_VALUE` or `DECOMPOSED_VALUE` is therefore positive evidence that the full braced name resolved before `set` performed its second variable-name lookup; it is not evidence that the braced raw name was absent.

Dynamic `namespace`, `proc`, `namespace which -command`, invocation, `rename` and exact namespace cleanup all succeeded for both producer families. Cross-producer values prove that similarly rendered names were not aliases.

## Expression semantics across BIG-IP Tcl contexts

The exact 387-byte expression payload ran in an `HTTP_REQUEST` iRule, a tmsh CLI script, an iApp implementation action and a triggered iCall script. All four BIG-IP-hosted contexts returned the same result bytes:

| Expression | Catch | Result |
| --- | ---: | --- |
| `"abcd" matches "a*"` | 0 | 1 |
| `"a*" matches "a*"` | 0 | 1 |
| `"a*" matches "abcd"` | 0 | 0 |
| `"abcd" matches "a.*"` | 0 | 0 |
| `"abcd" matches "bc"` | 0 | 0 |
| `"abcd" matches "abcd"` | 0 | 1 |
| `1 or 0 matches 0` | 0 | 1 |
| `not "abc" starts_with "a"` | 1 | `can't use non-numeric string as operand of "!"` |
| `not ("abc" starts_with "a")` | 0 | 0 |

For these discriminators, bare `matches` is whole-string glob matching: `a*` matches `abcd`, while `a.*` and `bc` do not. The earlier equality/non-substring pair alone was insufficient to distinguish equality from glob behavior. `not` binds to the following operand before `starts_with` unless parentheses group the string predicate.

The non-TMM fixtures loaded without error, the tmsh script emitted directly, the iApp emitted from its implementation action, and the iCall emitted from a triggered handler in `scriptd`. Raw outputs are [tmsh](evidence/r2286_20261006m/appliance/raw/contexts/cli-run.txt) and [iApp/iCall](evidence/r2286_20261006m/appliance/raw/contexts/scriptd-results.txt). iApp presentation/APL Tcl was not tested and must not inherit the implementation result.

## Activated procedure lifetime

The exact literal OLD and NEW provider sources were rejected at load because `namespace` is disabled in literal iRule source:

```text
01070151:3: Rule [...] error: ...:4: error: [command is disabled: "namespace"][namespace current]
```

Separately hashed repaired providers constructed `namespace current` dynamically. Those sources loaded; inside both OLD and NEW procedures it returned `::`. The caller used full absolute F5 routing to `/R2286_r2286m/lifetime/..._provider::incarnation`, passed an event-local `cell` by `upvar 1`, and returned both the procedure value and caller cell.

The authoritative lifetime control used fresh provider and caller object names that had never held a literal dependency. The provider was not attached to the VIP; the caller and backend rule were attached. The call target came from the byte-preserved request header.

| Phase | Provider config object | One actual argument | Two actual arguments | Coverage |
| --- | --- | --- | --- | --- |
| OLD active | Present, OLD body | catch 0; `OLD :: OLD_WRITE`; caller cell `OLD_WRITE` | catch 1; `wrong # args: should be "call incarnation <name>`; caller cell `CALLER_SEED` | 32 responses from each of `0:0`–`0:3` |
| After provider delete | Absent; `tmsh list` returned not found | Same OLD success and mutation | Same OLD arity error and unchanged cell | 32 per TMM in both phases |
| Same name recreated NEW | Present, NEW body/formals | catch 0; `NEW NEW_DEFAULT :: NEW_WRITE`; cell `NEW_WRITE` | catch 0; `NEW EXPLICIT_SUFFIX :: NEW_WRITE`; cell `NEW_WRITE` | 32 per TMM in both phases |
| Invalid replacement attempted | Existing NEW object retained | Same NEW default result | Same NEW explicit result | 32 per TMM in both phases |
| Same name recreated OLD | Present, OLD body/formals | OLD success restored | OLD arity error restored | 32 per TMM in both phases |

Deleting the fresh provider configuration therefore did not retire its already activated procedure from any reached TMM. Recreating the same owner path replaced the active body and formal list for fresh invocations. A failed replacement did not disturb the prior NEW callable. Recreating OLD replaced it again. These observations are about fresh calls after each configuration operation; they do not establish the lifetime of an already-entered suspended frame.

The invalid replacement was rejected exactly as follows:

```text
01070151:3: Rule [/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_provider_fresh] error: /R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_provider_fresh:4: error: [parse error: missing close-bracket][[list FAILED]
```

The original literal caller created a configuration dependency. Subsequent attempts using a dynamically constructed constant and a request-header target under that same caller object name still rejected provider deletion with:

```text
01070265:3: The rule (/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_provider) cannot be deleted because it is in use by a rule (/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_caller).
```

The raw phases named `provider_deleted`, `dynamic_provider_deleted` and `header_provider_deleted` followed those rejected delete attempts; the provider object was still present, so those responses are not callable-retirement evidence. They are retained as counterexamples rather than relabelled or discarded. The separately named `fresh_provider_deleted` phases follow an accepted delete plus explicit object-absence proof and are the authoritative deletion result.

No documented, bounded, event-valid mechanism was available to suspend execution inside an entered provider procedure without blocking TMM. The entered-frame replacement control is unsupported/unreached.

## Loader observations

- All 18 hexadecimal-transport iRule wrappers, the backend rule and all repaired lifetime controls loaded. Loader output and the resulting exact object configurations are retained under [load evidence](evidence/r2286_20261006m/appliance/raw/load/).
- The two exact literal provider bodies were rejected solely for literal `namespace current`; the dynamic repairs are distinct sources with distinct hashes.
- Both separately hashed invalid replacement bodies were rejected with `parse error: missing close-bracket`. Runtime traffic proved the prior callable remained active.
- The name payloads contain no literal NUL or non-ASCII byte in configuration source. These cases measure dynamic runtime values only and do not broaden the literal loader results in `BIGIP_RESULTS.md`.
- The follow-up did not repeat literal UTF-8 emoji/BMP loader cases, CRLF, physical continuation or embedded-NUL source controls. Those remain scoped to the earlier report and its hashes.

## Reproduction

The committed generators reject non-ASCII run identifiers, ensure payloads are ASCII/LF-only and NUL-free, emit hexadecimal transport, and record every source size and SHA-256.

```bash
PROBES=scripts/dev/bigip-probes/resolution-2286

python3 "$PROBES/generate-followup-controls.py" \
  --run r2286m --out /tmp/r2286m-fixtures \
  --vip 192.168.9.24 --vip-port 18101 \
  --backend 192.168.9.80 --backend-port 18100 \
  --source-commit 8ee8ad7c0df9eaab539f9663869ef664f20853e0

# Transfer without text conversion. On BIG-IP, compare every file with the
# manifest before any load, and start the raw /var/log/ltm tail first.
sha256sum /var/tmp/r2286m-fixtures/*
tmsh load sys config merge file /var/tmp/r2286m-fixtures/lab.conf
tmsh load sys config merge file /var/tmp/r2286m-fixtures/nul_counted_last.conf

# Run from dev.bragi0.com with a fresh source-port range per phase.
python3 "$PROBES/followup-traffic.py" \
  --vip 192.168.9.24 --port 18101 --run r2286m \
  --phase nul_counted_last --path /probe \
  --source-ip 192.168.9.80 --source-port-start 19000 \
  --requests 128 --expect-units 0:0,0:1,0:2,0:3 \
  --out /tmp/nul_counted_last.jsonl

# Transport the identical expression payload to distinct non-TMM contexts.
python3 "$PROBES/generate-followup-contexts.py" \
  --run r2286n --payload /tmp/r2286m-fixtures/expressions.tcl \
  --out /tmp/r2286n-contexts \
  --source-commit 8ee8ad7c0df9eaab539f9663869ef664f20853e0
bash "$PROBES/run-followup-contexts.sh" \
  r2286n /var/tmp/r2286n-contexts /var/tmp/r2286n-evidence

python3 "$PROBES/summarize-followup-evidence.py" \
  --traffic-dir /path/to/raw/traffic --out /tmp/decoded-results.json
```

The exact VIP configuration, including HTTP/TCP profiles and SNAT automap, is [virtual-initial.txt](evidence/r2286_20261006m/appliance/raw/config/virtual-initial.txt). The final bundle includes all loader messages, object snapshots, request identifiers, raw base64 protocol responses, server observations and continuous LTM output.

## Cleanup and retained evidence

All owned virtual servers, pools, nodes, Common test rules, fresh lifetime caller/provider, original caller, tmsh CLI script, iApp template/service, iCall script/handler and dev-host listeners were deleted by exact name. The context cleanup returned explicit not-found results. Ports 18100, 18102 and 18103 were absent after their recorded PIDs were stopped. The continuous LTM capture ran from `2026-10-06T15:13:32Z` through `2026-10-06T15:47:27Z` and its exact PID was stopped. Owned temporary fixture/evidence paths were removed from both hosts after local archive verification.

One appliance cleanup limitation remains. BIG-IP retained the original literal caller dependency after the caller object was absent. Replacing the caller with a no-call body, deleting it again, replacing the provider with a no-procedure body, and naming both rules in one delete command did not clear the reference. The following exact owned configuration remains and is not attached to a virtual server:

- partition `R2286_r2286m`;
- folder `/R2286_r2286m/lifetime`;
- rule `/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_provider`, whose final body contains only `RULE_INIT { set static::__tcl_lsp_2286_r2286m_cleanup_only 1 }`.

No listener, pool, node, caller or VIP remains. The earlier activated `incarnation` callable and cleanup-only static cell were not probed after the final neutral-body replacement, so their residual TMM runtime state is unknown. TMM was not restarted or reloaded to force retirement. The exact retained state is [remaining-state-final.txt](evidence/r2286_20261006m/appliance/raw/remaining-state-final.txt); rejected cleanup operations are [cleanup actions](evidence/r2286_20261006m/appliance/raw/cleanup-actions.txt), [stale-reference actions](evidence/r2286_20261006m/appliance/raw/cleanup-stale-reference-actions.txt) and [neutral-provider delete](evidence/r2286_20261006m/appliance/raw/cleanup-neutral-provider-delete.log).

Retained evidence:

- [Final appliance evidence archive](evidence/r2286_20261006m/r2286m-appliance-evidence-final.tgz) (`SHA-256 39109634e97d6780c27263ac2eee8027db682e34415a439ad57f092baf436d88`)
- [Continuous raw LTM log](evidence/r2286_20261006m/appliance/raw/ltm-raw.log)
- [Raw traffic and response summaries](evidence/r2286_20261006m/dev/traffic/)
- [Backend and dev-process evidence](evidence/r2286_20261006m/dev/)
- [Decoded byte-preserving result index](evidence/r2286_20261006m/decoded-results.json)
- [Fixtures, source hex and manifests](evidence/r2286_20261006m/fixtures/)
- [Local evidence SHA-256 inventory](evidence/r2286_20261006m/SHA256SUMS.local)

## Coverage limits

- The appliance has one TMM group with four units. Cross-blade, multi-group and HA peer behavior are untested.
- The runtime-name controls ran in TMM `HTTP_REQUEST`; only the exact expression payload was repeated in tmsh script, iApp implementation and triggered iCall.
- iCall was tested through one triggered handler. Periodic and perpetual handlers are untested.
- iApp presentation/APL parsing and embedded presentation callbacks are untested.
- Literal Unicode, emoji/grapheme logging, CRLF, continuation and embedded-NUL configuration-source behavior were not repeated in this follow-up.
- An already-entered suspended provider frame is unsupported/unreached.
- Persistence across TMM restart/reload is untested.
- The original provider, folder and partition remain because the appliance's retained caller dependency prevented supported deletion; residual callable/static runtime state after the neutral replacement is unmeasured.
