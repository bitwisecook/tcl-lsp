# BIG-IP iRule resolution results for #2286

> **Tested appliance: BIG-IP 21.1.0.1, build 0.0.26, Point Release 1. No separately named hotfix field was present in `tmsh show sys version`. FIPS Module: Cryptographic Module for BIG-IP.**

These are measured TMM iRule results from the active standalone lab appliance. They are not inferred from `tclsh`, tmsh scripts, iApps, the proposed static model, or the current implementation.

## Appliance and coverage

| Item | Measured value |
| --- | --- |
| Platform | BIG-IP Virtual Edition, platform Z100, Q35/KVM virtual hardware |
| Active device | Standalone, ACTIVE, `1/1 active` |
| TMM process | PID 28022, launched with `--npus 4` |
| Active roster | `0:0`, `0:1`, `0:2`, `0:3` |
| Roster proof | `init_identity` logged all four group/unit pairs; CMP-enabled `identity_group` returned exactly 64 responses from each pair; CMP-disabled returned 64/64 from `0:0` |
| Route domain | Default route domain 0 |
| VIP | `192.168.9.24:18086`, TCP and HTTP profiles |
| Client/backend | `dev.bragi0.com`, `192.168.9.80`; backend `192.168.9.80:18080` |
| Return path | Every test VIP used SNAT automap. The backend observed `192.168.9.24` as its peer and the client received the correlated response through the VIP. |

The complete inventory is [inventory.txt](evidence/r2286_20261006c/appliance/inventory.txt). The actual roster traffic is in [CMP-enabled identity JSONL](evidence/r2286_20261006c/dev/traffic-identity_group-cmp_yes.jsonl) and [CMP-disabled identity JSONL](evidence/r2286_20261006c/dev/traffic-identity_group-cmp_no.jsonl).

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

## Source identity and byte integrity

The tested checkout was branch `work` at source commit `5111900044ff998b20098ba5b68c8c25ff56fdf6`. The canonical run was `r2286_20261006c`.

| Fixture set | Manifest SHA-256 | Verification |
| --- | --- | --- |
| Canonical 53-rule set, `r2286_20261006c` | `6391afa87e7c03b9df2d9252625a7b0ebdee0850c830312a898f93fbbcc26e2c` | Every appliance file matched its manifest |
| ASCII-only exact-name cleanup | `69728f576b5dca85456295eaef86c9581bc2770e6e73a745c4f2f8723171f49e` | Appliance hashes matched |
| Isolated Unicode/LF/CRLF controls, `r2286_20261006d` | `83b598435bbaf35acf6d764ff0b5cc55d279cfddf72f0a2114b8511f812e21d7` | Every appliance file matched |
| Embedded-NUL controls, `r2286_20261006e` | `8f7fd3b7581fce66443397e281774f8b599cde1ead6a1a3f5d0b350ff48c34d9` | Every appliance file matched |

The canonical appliance hash list is [appliance-fixture-hashes.txt](evidence/r2286_20261006c/appliance/appliance-fixture-hashes.txt), its validation transcript is [manifest-verification.txt](evidence/r2286_20261006c/appliance/manifest-verification.txt), and the exact fixture bytes plus `.hex` copies are retained under [r2286_20261006c-fixtures](evidence/r2286_20261006c/appliance/r2286_20261006c-fixtures/). In particular, the supplied LF and CRLF files remained byte-distinct (`f747c42d…4331` and `9d608ed9…9e2c`).

## Principal observations

1. `RULE_INIT` ran once on each observed TMM for the tested rule creation: `0:0`, `0:1`, `0:2`, and `0:3`. A `static::` seed was then visible on every reached TMM.
2. An event write, unset, and recreate of the tested `static::` cell affected only the executing TMM. With target `0:2`, the other three TMMs retained the `RULE_INIT` seed throughout.
3. An ordinary global caused the owned VIP to execute only on unit 0 even with `cmp-enabled yes`. The requested CMP setting remained yes, but all seven 128-request global phases returned only unit 0. This is CMP demotion, not evidence that an ordinary global is shared among TMMs.
4. The same `static::` spelling collided across different rule owners. Creation A→B yielded `INIT_b`; creation B→A yielded `INIT_a`. Reversing attachment order did not change the value because the event priorities remained 101/102/110. Recreating only A changed every TMM's observer view to `INIT_a`; recreating only B changed every view to `INIT_b`.
5. F5 procedures were rule-owned. A's `call same` returned `A`, B's returned `B`, and caller routes `ruleName::same` and `/Common/ruleName::same` both reached A. A's `call caller_link x` allowed `upvar 1` to mutate the event-local `x` from `INITIAL` to `LINK_A`.
6. Literal `namespace`, `interp`, `package`, and `rename` command heads were rejected at rule load, while dynamically materialized/evaluated forms loaded and executed. Literal `proc` was invalid in event scope. An ordinary call head `same` was rejected even though an F5 top-level `proc same` existed; F5 `call` was required.
7. Literal UTF-8 source containing either precomposed `é` (`c3 a9`) or decomposed `e` plus combining acute (`65 cc 81`) was rejected with the same generic `braces are required around the expression` diagnostic. The result does not establish normalization because neither spelling reached TMM execution.
8. ASCII-only physical backslash-LF and backslash-CRLF sources both loaded. Both returned `{A B} {A B} {\n}`: the physical continuation became one space in braced and quoted words, while the braced literal backslash-n stayed two characters.
9. A literal NUL byte in the config source was rejected by the config parser. A dynamically materialized `41 00 42` value was length 3, scanned back as `410042`, worked as a variable name, and read back `VALUE` on every TMM.
10. Connection-local `event_local` set during `CLIENT_ACCEPTED` remained readable as `CLIENT_SEED` with catch status 0 in `HTTP_REQUEST`, `LB_SELECTED`, `SERVER_CONNECTED`, `HTTP_RESPONSE`, and `CLIENT_CLOSED` on all four TMMs.

## Initialization and state matrix

| Case | CMP mode and reached TMMs | Measured result |
| --- | --- | --- |
| `init_identity` | Rule load; all four TMM logs | `group=0`, units 0–3, `count=4` |
| `static_group` initial read | disabled: `0:0`; enabled: all `0:0`–`0:3` | Every reached TMM had `INIT_r2286_20261006c` |
| `static_group` write | enabled, target `0:2`, all four reached | Only `0:2` changed to `EVENT_0:2`; others remained `INIT_…` |
| `static_group` unset | enabled, target `0:2`, all four reached | First target request changed existence 1→0; later reads on `0:2` were `MISSING`; others retained `INIT_…` |
| `static_group` recreate | enabled, target `0:2`, all four reached | `0:2` became `RECREATED_0:2`; others retained `INIT_…` |
| `static_group` new initialization | delete/recreate exact rule | Four new `RULE_INIT` logs and four fresh `INIT_…` reads |
| `global` complete matrix | disabled: unit 0; enabled requested, only unit 0 reached | Write→`EVENT_0`, unset→missing, recreate→`RECREATED_0`; CMP-enabled distribution remained 128/128 on unit 0 in every phase |
| collision A→B creation | enabled, units 0–3 | Initial observer value `INIT_b` everywhere |
| collision B→A creation | enabled, units 0–3 | Initial observer value `INIT_a` everywhere |
| reverse attachment only | enabled, units 0–3 | Did not replace the current value; explicit event priority controlled writes |
| collision single-rule recreation | enabled, units 0–3 | Initial A→B `INIT_b`; recreate A → `INIT_a` everywhere; recreate B → `INIT_b` everywhere |

Raw state responses are in the `traffic-static-group-*`, `traffic-global-*`, and `traffic-collision-*` files under [dev evidence](evidence/r2286_20261006c/dev/). Single-rule recreation is separately retained under [single-recreate](evidence/r2286_20261006c/single-recreate/).

## F5 procedures and event frames

| Probe | Result on units 0, 1, 2, 3 |
| --- | --- |
| `procedure_a` local `call same` | `A` |
| `procedure_b` local `call same` | `B` |
| `procedure_a` `call caller_link x` | catch 0, return `LINK_A`, caller local became `LINK_A` |
| caller same-folder route to A | catch 0, `A` |
| caller absolute `/Common/...procedure_a::same` route | catch 0, `A` |
| A→B versus B→A creation/attachment | Same owner-correct results in every control |
| `event_frames` | All six events fired per request; later events read `CLIENT_SEED` with catch 0; backend status was 200 |

The procedure response set is [traffic-procedure-a-b.jsonl](evidence/r2286_20261006c/dev/traffic-procedure-a-b.jsonl); the owner-local results are in [raw LTM log](evidence/r2286_20261006c/appliance/ltm-raw.log). The forwarded-flow client responses are [CMP-enabled JSONL](evidence/r2286_20261006c/dev/traffic-event-frames-cmp-yes.jsonl), and [backend.log](evidence/r2286_20261006c/dev/backend.log) records the matching request IDs and SNAT peer `192.168.9.24`.

## Dynamic resolution matrix

Every row below loaded without warning, ran first and repeated on the CMP-disabled TMM, was cleaned by exact generated names, then ran on units 0, 1, 2, and 3 with 32 responses per unit. Unless a repeat result is stated, the repeat matched the first result. Returned `errorCode` availability was `UNAVAILABLE`, including caught errors, so no error-code value is inferred.

| Case | First measured result |
| --- | --- |
| `dynamic_namespace` | `11` |
| `dynamic_path_shadow` | Error: `bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which` |
| `dynamic_path_provider` | Same unsupported `namespace path` error |
| `dynamic_global_alias` | `11`; the proc's `global` alias reached the generated global cell |
| `dynamic_upvar` | `9` |
| `dynamic_upvar_unset` | `AGAIN` after target unset/recreate |
| `dynamic_uplevel` | `42` |
| `dynamic_upvar_absolute` | `CHANGED CHANGED` |
| `dynamic_upvar_zero` | `CHANGED` |
| `dynamic_uplevel_absolute` | `ABSOLUTE` |
| `dynamic_static_global_link` | `STATIC` |
| `dynamic_alias` | `ONE:FIXED` |
| `dynamic_alias_rename` | `1 {invalid command name "…_one"} ORIGINAL`; repeat failed earlier because `…_moved` already existed |
| `dynamic_rename_epoch` | `OLD NEW OLD`; repeat failed earlier because `…_moved` already existed |
| `dynamic_namespace_import` | `EXPORTED`; repeat: `can't import command "p": already exists` |
| `dynamic_namespace_delete` | `OLD NEW` |
| `dynamic_trace_scalar` | Read, write, unset callbacks in order; recreation did not retain the unset variable's trace |
| `dynamic_trace_upvar_array` | First `1 {}`; repeat recorded `{::…_arr k w}` from the pre-existing whole-array trace |
| `dynamic_trace_static` | Read then unset callbacks; recreation did not retain the unset cell's trace |
| `dynamic_package` | `8.4 0 8.4` (`provide Tcl`, require catch, required version) |
| `dynamic_package_resolution` | `1.0 1.0` |
| `dynamic_colon_names` | `7 7` for the tested multi-colon and canonical qualified spellings |
| `dynamic_read_failure` | Outer eval succeeded with value `1 {can't read "missing_r2286_20261006c": no such variable}` because the inner script caught the abrupt read |
| `dynamic_exists` | `0 0` without manufacturing contents |
| `dynamic_expr` | `3.5 14 3 1 {unknown math function "future_function"}` |
| `dynamic_word_bytes` | `{A B} {A B} {\n}` |

`dynamic_unicode_names` and `dynamic_unicode_variable_syntax` never ran: their outer iRules were rejected at load due to the literal Unicode bytes. This is a literal/dynamic boundary result, not evidence about runtime normalization or `$name` tokenization.

## Loader matrix and exact rejection classes

The canonical set produced 40 `ACCEPT`, 13 `REJECT`, no warnings, and no indeterminate loads. Complete status and per-rule logs are [load-driver-summary.tsv](evidence/r2286_20261006c/appliance/load-driver-summary.tsv) and [journals](evidence/r2286_20261006c/appliance/journals/).

| Cases | Verdict | Exact diagnostic class |
| --- | --- | --- |
| `minimal`, `identity`, `identity_group`, `init_identity`, `static`, `static_group`, `global` | ACCEPT | No warning |
| `collision_a`, `collision_b`, `collision_observer` | ACCEPT | No warning |
| `procedure_a`, `procedure_b`, `procedure_caller` | ACCEPT | No warning |
| All dynamic cases in the preceding table | ACCEPT | No warning |
| `event_frames` | ACCEPT | No warning |
| `encoding`, `dynamic_unicode_names`, `dynamic_unicode_variable_syntax`, `physical_words_lf`, `physical_words_crlf`, `runtime_cleanup` | REJECT | `braces are required around the expression` |
| `ordinary_literal` | REJECT | `undefined procedure: same` |
| `literal_namespace` | REJECT | `command is disabled: "namespace"` |
| `literal_interp` | REJECT | `command is disabled: "interp"` |
| `literal_package` | REJECT | `command is disabled: "package"` |
| `literal_rename` | REJECT | `command is disabled: "rename"` |
| `literal_trace` | REJECT | `wrong # args` for `trace info variable x` |
| `literal_proc` | REJECT | `command is not valid in the current scope` |

The isolated edge controls remove confounding source features:

| Control and source hash | Load/runtime result |
| --- | --- |
| ASCII binary control `ae43d52f…630b` | ACCEPT; returned hex `417c65` |
| Precomposed literal `3305392a…c4de` | REJECT; `braces are required around the expression` |
| Decomposed literal `4aae0373…812a` | REJECT; same diagnostic |
| Physical ASCII LF `c41c4375…297f` | ACCEPT; `{A B} {A B} {\n}` in both CMP modes |
| Physical ASCII CRLF `be4978b8…2ff5` | ACCEPT; identical runtime value in both CMP modes |
| Dynamic embedded NUL `7fd169e9…c3ca` | ACCEPT; decoded result `3 410042 0 VALUE 0 VALUE 1` on all four TMMs |
| Literal embedded NUL `59294325…126d` | REJECT; `Syntax Error … can't parse TCL script beginning with`; the diagnostic output stopped after the source bytes `set x "A` at the NUL boundary |

The raw edge-control evidence, including exact `.conf.hex` files and the literal-NUL loader bytes, is under [r2286_20261006de](evidence/r2286_20261006de/).

## Reproduction commands

```sh
PROBES=scripts/dev/bigip-probes/resolution-2286
RUN=r2286_20261006c

python3 "$PROBES/generate.py" --run "$RUN" --out /tmp/$RUN-fixtures
sha256sum /tmp/$RUN-fixtures/manifest.json /tmp/$RUN-fixtures/*.conf

# Transfer without text conversion, then compare every appliance file to manifest.json.
scp -r /tmp/$RUN-fixtures bigip:/var/tmp/
scp "$PROBES/appliance-rules.sh" bigip:/var/tmp/
ssh bigip 'sha256sum /var/tmp/r2286_20261006c-fixtures/*.conf'

# Start before the first rule creation.
ssh bigip 'tail -n 0 -F /var/log/ltm > /var/tmp/r2286-ltm-raw.log 2>&1 & echo $! > /var/tmp/r2286-capture.pid'

# Each case is isolated so one rejection cannot suppress another.
ssh bigip 'bash /var/tmp/appliance-rules.sh load /var/tmp/r2286_20261006c-fixtures /var/tmp/journal-minimal minimal'

python3 "$PROBES/render-lab.py" --run "$RUN" --vip 192.168.9.24 \
  --backend 192.168.9.80 \
  --rule /Common/__tcl_lsp_probe_2286_r2286_20261006c_minimal \
  --out /tmp/r2286-lab.conf

# Prove the exact pool/VIP/node absent, merge, and retain SNAT automap.
tmsh list ltm pool /Common/__tcl_lsp_probe_2286_r2286_20261006c_pool
tmsh list ltm virtual /Common/__tcl_lsp_probe_2286_r2286_20261006c_vs
tmsh list ltm node /Common/192.168.9.80
tmsh load sys config merge file /var/tmp/r2286-lab.conf

# Fresh TCP connections from the real client address.
python3 "$PROBES/traffic.py" --vip 192.168.9.24 --port 18086 \
  --run "$RUN" --source-ip 192.168.9.80 --source-port-start 20000 \
  --requests 256 --expect-units 0:0,0:1,0:2,0:3 \
  --out traffic-identity-group.jsonl

# Independent edge controls.
python3 "$PROBES/generate-load-controls.py" --run r2286_20261006d --out /tmp/load-controls
python3 "$PROBES/generate-nul-controls.py" --run r2286_20261006e --out /tmp/nul-controls
```

The exact attachment commands, requested/actual CMP state, and full VIP configurations are in [attachments](evidence/r2286_20261006c/appliance/attachments/). The committed [run-appliance-matrix.sh](run-appliance-matrix.sh) and [traffic-control.py](traffic-control.py) reproduce the bounded sequencing used here.

## Cleanup and retained evidence

The supplied `runtime_cleanup` could not load because its outer source contains the rejected Unicode names. The separately hashed ASCII-only cleanup names only the generated ASCII runtime objects. It ran after every dynamic case on the CMP-disabled TMM and then across `0:0`–`0:3`; the final post-collision cleanup also covered all four pairs. Unicode runtime names were never created because both Unicode rules rejected at load.

All owned virtual servers, pools, implicit nodes, and rule objects were removed by exact name. Final `tmsh list` calls returned explicit “was not found” results. The backend, artifact server, traffic controller, log tails, and evidence receivers were stopped by recorded PID; ports 18080–18083 were absent. See [main cleanup](evidence/r2286_20261006c/appliance/cleanup.txt), [final runtime cleanup](evidence/r2286_20261006c/final-runtime-cleanup/), [edge cleanup](evidence/r2286_20261006de/appliance/cleanup.txt), and [dev process cleanup](evidence/r2286_20261006c/dev/process-cleanup.txt).

The retained evidence bundles are:

- [Compressed complete bundle](evidence/resolution-2286-bigip-evidence-20261006.tar.gz) (`SHA-256 181ed1184d36c4e03f9ef484b84482cd4018d74d8e22e7279d3fc5530b8eb2bb`)
- [Main appliance and traffic evidence](evidence/r2286_20261006c/)
- [Independent LF/CRLF, Unicode, and NUL evidence](evidence/r2286_20261006de/)
- [Continuous raw main `/var/log/ltm`](evidence/r2286_20261006c/appliance/ltm-raw.log)
- [Raw backend observations](evidence/r2286_20261006c/dev/backend.log)

## Coverage limits

- The topology has one TMM group with four units. Cross-blade or multi-group behavior is not tested.
- The literal Unicode rules did not load. Precomposed/decomposed runtime name equality and original `$name` tokenization are therefore unsupported/unreached on this build, not negative runtime results.
- The supplied physical LF/CRLF rules also did not load because each contains literal Unicode. Only the separately hashed ASCII-only physical controls reached runtime.
- `static_group` supplied the complete group/unit matrix. The unit-only `static` rule loaded but was not separately trafficked.
- `identity_group` supplied the roster. The unit-only `identity` rule loaded but was not separately trafficked.
- No extra folders were created, so optional cross-folder F5 procedure routing was not tested.
- Standalone Tcl controls were not used as substitutes for any TMM result.

## Post-test appliance state

The post-reboot state is recorded in [phonehome-enforced-verification.txt](evidence/r2286_20261006c/appliance/phonehome-enforced-verification.txt): `auto-check` and `auto-phonehome` are disabled, `f5_update_checker` is boot-disabled and down, `telemd` is unregistered with no process, the root crontab has no `phonehome_upload` entry, both executable paths are absent, `/usr` is read-only, and `/config/startup` invokes the idempotent enforcement script after future boots.
