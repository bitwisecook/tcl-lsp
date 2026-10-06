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
| VIPs | `192.168.9.24:18086` (canonical), `:18087` (`/Common` procedure/Unicode), and `:18088` (temporary-partition procedure); TCP and HTTP profiles |
| Client/backend | `dev.bragi0.com`, `192.168.9.80`; backends `192.168.9.80:18080` and `:18084` |
| Return path | Every test VIP used SNAT automap. The `r2286i` backend recorded 2,496 correlated requests and every peer was `192.168.9.24`; the client received each correlated response through the VIP. |

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

The tested checkout was branch `work`. The canonical run `r2286_20261006c` used source commit `5111900044ff998b20098ba5b68c8c25ff56fdf6`; the procedure/folder/partition and Unicode run `r2286i` used `b5304db343a21a7f411a527078fbcf8b3fb5fbe2`; the five-context run `r2286l` used `d04406a0ce9e9fd2ceb5e91c39d602eb9246ef57`.

| Fixture set | Manifest SHA-256 | Verification |
| --- | --- | --- |
| Canonical 53-rule set, `r2286_20261006c` | `6391afa87e7c03b9df2d9252625a7b0ebdee0850c830312a898f93fbbcc26e2c` | Every appliance file matched its manifest |
| ASCII-only exact-name cleanup | `69728f576b5dca85456295eaef86c9581bc2770e6e73a745c4f2f8723171f49e` | Appliance hashes matched |
| Isolated Unicode/LF/CRLF controls, `r2286_20261006d` | `83b598435bbaf35acf6d764ff0b5cc55d279cfddf72f0a2114b8511f812e21d7` | Every appliance file matched |
| Embedded-NUL controls, `r2286_20261006e` | `8f7fd3b7581fce66443397e281774f8b599cde1ead6a1a3f5d0b350ff48c34d9` | Every appliance file matched |
| Procedure/folder/partition and Unicode controls, `r2286i` | `8f5c9f385b226b5f7da6885ede6a28775a68cdfea7ed44fb28fb213c73f6e6fd` | Every appliance file matched the per-file SHA-256 manifest before load |
| Five-context 92-case matrix, `r2286l` | `16ea0ab2150d247f7f3cd50b95f10d14282753b9b792a06490639705d785e483` | All six generated wrappers matched their per-file SHA-256 manifest before load |

The canonical appliance hash list is [appliance-fixture-hashes.txt](evidence/r2286_20261006c/appliance/appliance-fixture-hashes.txt), its validation transcript is [manifest-verification.txt](evidence/r2286_20261006c/appliance/manifest-verification.txt), and the exact fixture bytes plus `.hex` copies are retained under [r2286_20261006c-fixtures](evidence/r2286_20261006c/appliance/r2286_20261006c-fixtures/). In particular, the supplied LF and CRLF files remained byte-distinct (`f747c42d…4331` and `9d608ed9…9e2c`).

## Principal observations

1. `RULE_INIT` ran once on each observed TMM for the tested rule creation: `0:0`, `0:1`, `0:2`, and `0:3`. A `static::` seed was then visible on every reached TMM.
2. An event write, unset, and recreate of the tested `static::` cell affected only the executing TMM. With target `0:2`, the other three TMMs retained the `RULE_INIT` seed throughout.
3. An ordinary global caused the owned VIP to execute only on unit 0 even with `cmp-enabled yes`. The requested CMP setting remained yes, but all seven 128-request global phases returned only unit 0. This is CMP demotion, not evidence that an ordinary global is shared among TMMs.
4. The same `static::` spelling collided across different rule owners. Creation A→B yielded `INIT_b`; creation B→A yielded `INIT_a`. Reversing attachment order did not change the value because the event priorities remained 101/102/110. Recreating only A changed every TMM's observer view to `INIT_a`; recreating only B changed every view to `INIT_b`.
5. F5 top-level procedures have local ownership and a separate cross-iRule route. Local `call same` selected the calling rule's own proc. `call rule::proc` selected a partition-root rule, while a folder-qualified rule required its full folder path; the `rule` prefix is namespace-like syntax but is not an ordinary Tcl namespace or command.
6. Literal `namespace`, `interp`, `package`, and `rename` command heads were rejected at rule load, while dynamically materialized/evaluated forms loaded and executed. Literal `proc` was invalid in event scope. An ordinary call head `same` was rejected even though an F5 top-level `proc same` existed; F5 `call` was required.
7. Literal UTF-8 acceptance was byte-pattern dependent rather than general Unicode support. Precomposed `é`, decomposed `e` plus combining acute, BMP symbols, variation-selector, ZWJ and keycap sequences rejected with `braces are required around the expression`; literal 😀, 👍🏽 and 🇳🇿 loaded and logged on every TMM. Dynamic bytearrays preserved every tested UTF-8 byte sequence but TMM `string length` counted bytes and rendered the dynamic bytes as mojibake.
8. ASCII-only physical backslash-LF and backslash-CRLF sources both loaded. Both returned `{A B} {A B} {\n}`: the physical continuation became one space in braced and quoted words, while the braced literal backslash-n stayed two characters.
9. A literal NUL byte in the config source was rejected by the config parser. A dynamically materialized `41 00 42` value was length 3, scanned back as `410042`, worked as a variable name, and read back `VALUE` on every TMM.
10. Connection-local `event_local` set during `CLIENT_ACCEPTED` remained readable as `CLIENT_SEED` with catch status 0 in `HTTP_REQUEST`, `LB_SELECTED`, `SERVER_CONNECTED`, `HTTP_RESPONSE`, and `CLIENT_CLOSED` on all four TMMs.
11. The same 92 byte-controlled parser cases ran in TMM `RULE_INIT`, tmsh `cli script`, iApp implementation, iCall script, and appliance `tclsh` controls. The four F5-hosted contexts agreed on the tested grammar, newline, comment, expression and numeral cases, while their command surfaces/environments differed. iApp and iCall were byte-identical for all 92 cases and reported environment.
12. `if {1}{#do something}` succeeds in every tested F5-hosted context and leaves the sentinel unchanged because the body is a comment through end-of-body; appliance `tclsh8.4` and `tclsh8.5` reject it with `extra characters after close-brace`. Bare `matches` measured as exact equality in the discriminating pair, and `not "abc" starts_with "a"` applies `not` first and errors on the non-numeric string.

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

The original owner-local controls and the expanded `r2286i` folder/partition matrix reached units 0, 1, 2 and 3. Every 128-request phase produced exactly 32 responses from each actual unit. The three literal-emoji phases produced 16 from each unit.

| Probe | Measured result |
| --- | --- |
| Same rule, local `call same` | A's proc returned `A`; B's proc returned `B` |
| `call caller_link x` | catch 0, return `LINK_A`; `upvar 1` changed the event-local caller variable to `LINK_A` |
| Caller loaded before providers | Caller loaded successfully; every cross-rule call failed at runtime with `proc identify not found`; the caller-local proc worked |
| Partition-root relative `rule::identify` | Resolved the rule at the caller's partition root |
| Folder caller, leaf-only `rule::identify` | Did **not** resolve the rule in the current folder; failed `proc identify not found` |
| Folder or nested-folder absolute `/partition/folder/rule::identify` | Reached the named rule; same-folder, sibling, parent, nested and cross-partition controls succeeded |
| Temporary-partition caller to `/Common/...::identify` | Succeeded; `/Common` was visible from the non-Common partition |
| `::rule::identify` | Failed `proc ::…::identify - invalid namespace` |
| `namespace current` inside provider proc | `::` for local and cross-rule calls in all folders/partitions |
| Dynamic `namespace which -command $target` | Empty for every F5 `call` target, including targets that immediately succeeded |
| Provider creation forward versus reverse | No result changed |
| Delete provider config after successful activation | Calls continued to succeed on all four TMMs; deleting the config object did not retract the activated procedure from the running TMM interpreters |
| Recreate deleted provider | Calls continued to succeed on all four TMMs |
| Common VIP attaching a temporary-partition rule | Rejected: `01070726:3: A virtual server may only reference rules in the same partition or the common partition (/Common/__tcl_lsp_probe_2286_r2286i_vs:/R2286_r2286i/__tcl_lsp_probe_2286_r2286i_caller)` |
| Temporary-partition VIP attaching a Common rule | Accepted and reached all four TMMs |
| `event_frames` | All six events fired per request; later events read `CLIENT_SEED` with catch 0; backend status was 200 |

These results support a distinct F5 procedure-owner path, not ordinary Tcl namespace resolution. The spelling `foo::proc1` is valid for an iRule named `foo` at the caller's partition root. The counterexamples are load-order-independent runtime absence before the provider is activated, no same-folder leaf search, `namespace current == ::`, empty `namespace which`, rejection of a Tcl-rooted `::foo::proc1`, and procedure retention after deletion of the provider's configuration object. Absolute object paths are required to address folder-contained rules unambiguously.

The original procedure results are in [canonical raw LTM log](evidence/r2286_20261006c/appliance/ltm-raw.log). Expanded proof is in the [r2286i raw LTM log](evidence/r2286_20261006hi/appliance/ltm-raw.log), [attachment transcripts](evidence/r2286_20261006hi/appliance/attachments/), [traffic responses](evidence/r2286_20261006hi/dev/traffic/), and [backend observations](evidence/r2286_20261006hi/dev/backend.log).

## Literal and dynamic Unicode logging

Each literal fixture contains the exact UTF-8 bytes named below. Files were transferred without text conversion, checked against [manifest.json](evidence/r2286_20261006hi/appliance/manifest.json), and loaded independently.

| Payload | UTF-8 hex | Literal load/runtime result | Dynamic bytearray result |
| --- | --- | --- | --- |
| precomposed `é` (U+00E9) | `c3a9` | REJECT, `braces are required around the expression` | preserved; length 2 |
| decomposed `e` + U+0301 | `65cc81` | REJECT, same diagnostic | preserved; length 3 |
| © | `c2a9` | REJECT, same diagnostic | preserved; length 2 |
| ☃ | `e29883` | REJECT, same diagnostic | preserved; length 3 |
| ❤ | `e29da4` | REJECT, same diagnostic | preserved; length 3 |
| 😀 | `f09f9880` | ACCEPT; visually logged; length 4 | preserved; length 4 |
| ❤️ | `e29da4efb88f` | REJECT, same diagnostic | preserved; length 6 |
| 👍🏽 | `f09f918df09f8fbd` | ACCEPT; visually logged; length 8 | preserved; length 8 |
| 👩🏽‍💻 | `f09f91a9f09f8fbde2808df09f92bb` | REJECT, same diagnostic | preserved; length 15 |
| family ZWJ sequence | `f09f91a8e2808df09f91a9e2808df09f91a7e2808df09f91a6` | REJECT, same diagnostic | preserved; length 25 |
| 🇳🇿 | `f09f87b3f09f87bf` | ACCEPT; visually logged; length 8 | preserved; length 8 |
| rainbow-flag ZWJ sequence | `f09f8fb3efb88fe2808df09f8c88` | REJECT, same diagnostic | preserved; length 14 |
| 1️⃣ | `31efb88fe283a3` | REJECT, same diagnostic | preserved; length 7 |

The three accepted literal rules ran on all four TMMs. The dynamic rule constructed each value with `binary format H*`; every `binary scan ... H*` result exactly equalled its source hex on all four TMMs in both provider-order phases. TMM `string length` counted the encoded bytes, not Unicode scalar values or grapheme clusters. Literal accepted emoji appeared visually in syslog; dynamically constructed UTF-8 bytearrays were logged as byte-oriented mojibake even though their hex was intact. These rows do not justify a general acceptance rule for BMP versus non-BMP text: they establish only the exact measured byte sequences.

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

## Five-context F5 Tcl divergence matrix

Run `r2286l` evaluated the canonical 34 cases from `suites/10-context-parity.cases` plus 58 separately identified additions from [deep-context.cases](deep-context.cases): 92 unique case IDs. The generator materialised each source from hex so LF, CRLF, physical backslash-newline, braces, comment placement and literal escapes were byte-controlled. In every wrapper the hex was converted to a bytearray and evaluated dynamically; these rows measure runtime parsing and remain separate from the literal rule-load results above. TMM cases were split across ten iRules; every chunk emitted exactly once on each actual TMM unit (`0:0` through `0:3`). tmsh, iApp, iCall, `tclsh8.4` and `tclsh8.5` each emitted all 92 results plus one environment row.

| Surface | Measured environment | 92-case relationship |
| --- | --- | --- |
| TMM iRule `RULE_INIT` | Tcl 8.4.6; `exec` absent; Tcl package only; fabricated BIG-IP `tcl_platform`; `tmsh::version` absent at runtime | Same tested F5 grammar as the other three F5 contexts; distinct command/environment surface |
| tmsh `cli script` | Tcl 8.4.6; reported `tcl_patchLevel=UNSET`; `exec` works; Tcl package only; empty `tcl_platform`; tmsh 21.1.0.1 | Same tested F5 grammar; distinct environment |
| iApp implementation | Tcl 8.4.6; `exec` works; 95 commands; populated 32-bit Linux-like platform; large ambient package set; tmsh 21.1.0.1 | Byte-identical to iCall for every case and reported environment |
| iCall script | Tcl 8.4.6; same reported command, package and platform surface as iApp; executed by a triggered handler in `scriptd` | Byte-identical to iApp for every case and reported environment |
| appliance `tclsh8.4` | Tcl 8.4.13; 85 commands | Control only; multiple grammar and surface differences |
| appliance `tclsh8.5` | Tcl 8.5.13; 92 commands | Control only; multiple grammar, index and surface differences |

Selected discriminators:

| Exact source or case | Four F5-hosted contexts | Host controls |
| --- | --- | --- |
| `set m before; if {1}{#do something}; set m` | rc 0, `before`; `#` comments through the end of the braced body | 8.4/8.5 rc 1, `extra characters after close-brace` |
| Same body with LF before `set m after` | rc 0, `after` | 8.4/8.5 same close-brace error |
| `if {1}` then body on next LF line | rc 0, `after` | 8.4/8.5 `wrong # args: no script following "1" argument` |
| Same with CRLF | rc 0, `after`; identical to LF | 8.4/8.5 same wrong-args result |
| Physical backslash-LF between condition and body | rc 0, `after` | rc 0, `after` |
| Backslash-space-LF | rc 0, empty result | 8.4/8.5 rc 1, close-brace error |
| `expr {"abcd" matches "abcd"}` | `1` | operator rejected |
| `expr {"abcd" matches "bc"}` | `0`; bare `matches` is exact equality for this discriminating pair | operator rejected |
| `expr {1 or 0 matches 0}` | `1`; `matches` binds more tightly than `or` in this probe | operator rejected |
| `expr {not "abc" starts_with "a"}` | rc 1, `can't use non-numeric string as operand of "!"`; `not` binds to the string before `starts_with` | operator syntax rejected |
| `lindex {a b} end+1` | rc 1, `bad index "end+1": must be integer or end?-integer?` | 8.4 same error; 8.5 rc 0, empty |
| `switch -- a {a{set m yes} ...}` | rc 1, `extra switch pattern with no body` | same; script implicit word joining does not apply inside the switch list |
| `trace add variable q read {list}` | accepted, rc 0 | accepted, rc 0 |

The current codebase comparison exposes two concrete modelling corrections. First, `BigIpExecutionContext` has no iCall variant even though the measured iCall surface is real and separately invokable; it must not be silently treated as iApp merely because this build produced identical rows. Second, current documentation/runtime comments mark bare `matches` semantics and `not` precedence as unmeasured; this run measures both discriminators above. The APL presentation DSL and Tcl callbacks embedded in APL remain unmeasured and must not inherit the implementation/iCall results.

The execution mechanisms follow F5's primary documentation for [iRule `call`](https://clouddocs.f5.com/api/irules/call.html), [tmsh CLI scripts](https://clouddocs.f5.com/cli/tmsh-reference/v16/modules/cli/cli_script.html), [iApp templates](https://clouddocs.f5.com/cli/tmsh-reference/v16/modules/sys/sys_application_template.html), [iCall scripts](https://clouddocs.f5.com/cli/tmsh-reference/v15/modules/sys/sys_icall_script.html), [iCall events](https://clouddocs.f5.com/cli/tmsh-reference/latest/modules/sys/sys_icall_event.html), and [triggered handlers](https://clouddocs.f5.com/cli/tmsh-reference/v16/modules/sys/sys_icall_handler_triggered.html). Raw results are [TMM LTM](evidence/r2286_20261006l/ltm-raw.log), [tmsh](evidence/r2286_20261006l/cli-run.txt), [iApp and iCall](evidence/r2286_20261006l/scriptd-results.txt), [tclsh8.4](evidence/r2286_20261006l/host-tclsh8.4.txt), and [tclsh8.5](evidence/r2286_20261006l/host-tclsh8.5.txt).

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

# Folder/partition procedure ownership and literal/dynamic Unicode.
python3 "$PROBES/generate-procedure-scope-controls.py" --run r2286i \
  --out /tmp/r2286i-fixtures
python3 "$PROBES/render-procedure-scope-labs.py" --run r2286i \
  --vip 192.168.9.24 --backend 192.168.9.80 --backend-port 18084 \
  --common-vip-port 18087 --partition-vip-port 18088 \
  --out /tmp/r2286i-lab.conf
sha256sum /tmp/r2286i-fixtures/manifest.json /tmp/r2286i-fixtures/*.conf \
  /tmp/r2286i-lab.conf
# On BIG-IP, with continuous ltm capture already active and the dev.bragi0.com
# backend plus traffic-control endpoints already listening:
bash run-procedure-scope-matrix.sh r2286i /var/tmp/r2286i-evidence \
  /var/tmp/r2286i-fixtures /var/tmp/r2286i-lab.conf \
  http://192.168.9.80:18082/run http://192.168.9.80:18083/run

# Identical byte-controlled sources in TMM, tmsh, iApp, iCall and host tclsh.
python3 "$PROBES/generate-deep-context.py" --run r2286l \
  --canonical scripts/dev/bigip-probes/suites/10-context-parity.cases \
  --additional "$PROBES/deep-context.cases" --out /tmp/r2286l-fixtures
sha256sum /tmp/r2286l-fixtures/manifest.json /tmp/r2286l-fixtures/*
bash run-deep-context.sh r2286l /var/tmp/r2286l-fixtures \
  /var/tmp/r2286l-evidence
```

The exact attachment commands, requested/actual CMP state, and full VIP configurations are in [attachments](evidence/r2286_20261006c/appliance/attachments/). The committed [run-appliance-matrix.sh](run-appliance-matrix.sh) and [traffic-control.py](traffic-control.py) reproduce the bounded sequencing used here.

## Cleanup and retained evidence

The supplied `runtime_cleanup` could not load because its outer source contains the rejected Unicode names. The separately hashed ASCII-only cleanup names only the generated ASCII runtime objects. It ran after every dynamic case on the CMP-disabled TMM and then across `0:0`–`0:3`; the final post-collision cleanup also covered all four pairs. Unicode runtime names were never created because both Unicode rules rejected at load.

All owned virtual servers, pools, named nodes, partitions, folders, rules, CLI scripts, iApp templates/services, iCall scripts/handlers and generated events were removed by exact name. Final `tmsh list` calls returned explicit “was not found” results. The backend, artifact server, traffic controllers, log tails and evidence receivers were stopped by recorded PID; their owned listener ports were absent. See [main cleanup](evidence/r2286_20261006c/appliance/cleanup.txt), [final runtime cleanup](evidence/r2286_20261006c/final-runtime-cleanup/), [edge cleanup](evidence/r2286_20261006de/appliance/cleanup.txt), [procedure/partition cleanup](evidence/r2286_20261006hi/appliance/cleanup-verification.txt), [five-context cleanup](evidence/r2286_20261006l/cleanup-verification.txt), [final live cleanup audit](evidence/r2286_20261006l/final-live-cleanup-audit.txt), and [dev process cleanup](evidence/r2286_20261006c/dev/process-cleanup.txt).

The retained evidence bundles are:

- [Compressed complete bundle](evidence/resolution-2286-bigip-evidence-20261006.tar.gz) (`SHA-256 181ed1184d36c4e03f9ef484b84482cd4018d74d8e22e7279d3fc5530b8eb2bb`)
- [Main appliance and traffic evidence](evidence/r2286_20261006c/)
- [Independent LF/CRLF, Unicode, and NUL evidence](evidence/r2286_20261006de/)
- [Continuous raw main `/var/log/ltm`](evidence/r2286_20261006c/appliance/ltm-raw.log)
- [Raw backend observations](evidence/r2286_20261006c/dev/backend.log)
- [Procedure/folder/partition appliance bundle](evidence/r2286_20261006hi/r2286i-appliance-evidence.tgz) (`SHA-256 1fb4f097c33965752154dff4497eee3a37a6e4296173e9a068ae0fc06402d9e2`)
- [Procedure/folder/partition client/backend bundle](evidence/r2286_20261006hi/r2286i-dev-evidence-v2.tgz) (`SHA-256 d00330e986e00fbd30fa88c6d766f8cacb475a77cfa14f0dfdd7b2fd4b6cd875`)
- [Five-context appliance bundle](evidence/r2286_20261006l/r2286l-appliance-evidence.tgz) (`SHA-256 f635b42c411e64a12f95c12d87cad26bd9d3c6eca68c6f2c2353d51982c29dc9`)
- [Continuous raw procedure/Unicode `/var/log/ltm`](evidence/r2286_20261006hi/appliance/ltm-raw.log)
- [Continuous raw five-context `/var/log/ltm`](evidence/r2286_20261006l/ltm-raw.log)

## Coverage limits

- The topology has one TMM group with four units. Cross-blade or multi-group behavior is not tested.
- The canonical literal-name rules did not load. Precomposed/decomposed runtime name equality and original `$name` tokenization are therefore unsupported/unreached on this build, not negative runtime results. The later logging controls measured only string values, not Unicode identifier equality.
- The supplied physical LF/CRLF rules also did not load because each contains literal Unicode. Only the separately hashed ASCII-only physical controls reached runtime.
- `static_group` supplied the complete group/unit matrix. The unit-only `static` rule loaded but was not separately trafficked.
- `identity_group` supplied the roster. The unit-only `identity` rule loaded but was not separately trafficked.
- APL presentation parsing and Tcl callbacks embedded inside APL were not executed. They remain unsupported/unreached and must not inherit iApp implementation results.
- iCall was measured through one triggered handler and one scriptd execution. Periodic/perpetual handlers and role-restricted execution were not tested.
- Only BIG-IP 21.1.0.1 build 0.0.26 was tested. Version-introduction/removal claims across the BIG-IP release line remain unsupported.
- Provider deletion tested the running TMM retention observed on this roster; a TMM restart/reload after deletion was not performed, so persistence across TMM restart is unsupported.
- The TMM trace rows share an interpreter across the ten `RULE_INIT` rules. Accumulated trace registrations make trace-list multiplicity a test-state result, not a dialect claim; trace syntax acceptance is measured.
- Standalone Tcl controls were not used as substitutes for any TMM result.

## Post-test appliance state

The final live state is recorded in [phonehome-update-telemetry-final-verification.txt](evidence/r2286_20261006l/phonehome-update-telemetry-final-verification.txt): `auto-check` and `auto-phonehome` are disabled; `f5_update_checker` is boot-disabled and down; `telemd` is unregistered; there are no `telemd`, `phonehome_upload`, or `updatecheck` processes; root cron has no uploader or update-check entry; all three active executable paths are absent and their retained copies are non-executable; `/usr` is read-only; and `/config/startup` invokes the idempotent enforcement script after future boots. The remaining persistent appliance state is this deliberately retained call-out suppression and its dated backup/log under `/config/phonehome-disabled`; no test traffic object or test process remains.
