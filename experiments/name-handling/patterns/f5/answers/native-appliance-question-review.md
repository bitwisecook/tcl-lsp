# Native appliance-question evidence review

All five appliance questions have concrete owned materialisations and independent capability boundaries below. The public rule names motivate an idiom; only the retained provider/caller inputs were loaded. This review records the bounded variants reached in native TMM contexts and keeps the wider question scope explicit. No Tcl stub substitutes for native F5 `call`.

The [app3 report](nh20261012app3/ANSWERS.md) and [records](nh20261012app3/records.json) retain provider/caller definitions, applied configurations, exact dynamic invocation sources, load/attachment receipts, version/build, actual TMM units, result bytes and cleanup. `RULE_INIT` and native-procedure calls ran on four observed units; sixteen fresh traffic connections ran on actual unit `0:0`. A primary pointer below represents one invocation only. Full per-unit/invocation inventories remain in JSON.

| Appliance question | Reached bounded native materialisations | Remaining scope |
| --- | --- | --- |
| `corpus-f5-rule-path` | `absolute-other`, `absolute-provider`, `bare-local`, `missing-proc`, `missing-rule`, `relative-provider` | app3 measures Common bare-local, relative-provider, absolute-provider, other provider and missing route/proc. Existing canonical owner-route supplies folders and cross-partition controls; these exact configurations have independent scope. |
| `corpus-f5-computed-target` | `computed-target` | A variable/list-built owned Common target reaches the same OLD native provider result as its literal call-route control in all three TMM contexts. Other partitions/folders/target mutations are separate variants. |
| `corpus-f5-prefix-arguments` | `list-prefix-arguments`, `space-joined-near-miss` | Fixed/list-built arguments preserve spaces, braces and namespace separator data in the owned native provider. The space-joined near miss returns an observed wrong-argument-count result. Arbitrary event/profile argument injection remains outside this control. |
| `corpus-f5-rule-lifetime` | `provider-lifetime` | Sixteen actual traffic invocations observe OLD then NEW provider definitions and calls after provider detachment. RULE_INIT/procedure observations are initial OLD controls. Existing activated replacement proof adds deletion/recreation/formal replacement; entered suspended frames and restart/reload remain unmeasured. |
| `corpus-f5-storage-context` | `connection-identity`, `storage-read-write` | Owned cross-rule native cells return scope/level and an incrementing static counter; fresh traffic invocations share the actual worker cell. Broader namespace/regular-global/connection storage is measured by the separate persistence run, whose review is linked below. |

## Exact capability boundaries

CLI, iApp implementation, iCall, actual APL and stock Tcl each independently evaluate the retained dynamic command `call ::<run-owned-absent-target>` and return code1 `invalid command name "call"`. These results establish the native-call boundary on those exact surfaces; they neither supply native F5 semantics nor classify every Tcl operation unsupported. TMM call-provider-missing responses are separate and demonstrate native routing exists.

| Context | Capability receipt |
| --- | --- |
| `tmsh-cli-script` | [records](nh20261012cap2/records.json) `/237` SHA-256 `c1dd512f373200985d68039335a656fe1ae86db54fb831ce44a4083cae888ad2` |
| `iapp-implementation` | [records](nh20261012cap2/records.json) `/16` SHA-256 `c1dd512f373200985d68039335a656fe1ae86db54fb831ce44a4083cae888ad2` |
| `icall` | [records](nh20261012cap2/records.json) `/33` SHA-256 `c1dd512f373200985d68039335a656fe1ae86db54fb831ce44a4083cae888ad2` |
| `iapp-apl` | [records](nh20261012aplcap1/records.json) `/16` SHA-256 `141d74bc12631a488c5fa964751c7d9ea9cdc9573f969a84372a2b8e862781e1` |
| `stock-8.4` | [records](nh20261012cap2/records.json) `/50` SHA-256 `c1dd512f373200985d68039335a656fe1ae86db54fb831ce44a4083cae888ad2` |
| `stock-8.5` | [records](nh20261012cap2/records.json) `/67` SHA-256 `c1dd512f373200985d68039335a656fe1ae86db54fb831ce44a4083cae888ad2` |

## Existing canonical N11/N13 controls

- [`naming.bigip.procedure-owner-route`](../../../../../docs/design/analysis/name-resolution-proofs/bigip-procedure-owner-route.md) retains same-tail owners, bare local and relative/absolute folder/partition routes, caller/provider creation order, and calls after provider configuration deletion. Successful F5 call targets can be absent from `namespace which`; this proof does not turn native catalogues into Tcl procedure slots.
- [`naming.bigip.static-state-tmm-locality`](../../../../../docs/design/analysis/name-resolution-proofs/bigip-static-state-tmm-locality.md) records all four actual RULE_INIT seeds, ordered static write/unset/recreate on unit `0:2`, unaffected other units, recreation, and cross-rule static spelling collision. Ordinary-global traffic there reached only unit `0:0`; no other-worker sharing is inferred.
- [`naming.bigip.activated-procedure-replacement`](../../../../../docs/design/analysis/name-resolution-proofs/bigip-activated-procedure-replacement.md) records accepted provider deletion with OLD calls still active, NEW formals/body replacement, failed replacement retaining NEW, and OLD recreation. Entered suspended frames and restart/reload after deletion remain outside the reached evidence.

| Exact retained input/raw control | SHA-256 |
| --- | --- |
| [retained control](../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/journals/procedure_a/created-procedure_a.conf) | `409718fab22a0c4070d908b8a0a7a2a3ee65d9165c2b65e1d6e6bacc0b9024d7` |
| [retained control](../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/journals/static_group/created-static_group.conf) | `4ad2a1db52dda64f4347bca37ee71a324c8a8f4dfe3122e892808dd439e03f49` |
| [retained control](../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/ltm-raw.log) | `d1b804b6eb1587c7cbff4e71f267cac9e8f3a4d1a63feb3d41943cda9a80c48b` |
| [retained control](../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/ltm-raw.log) | `cdf50547d7d4e025c778c295b95c293719d2e9699c1a28f44edb474d096ff691` |
| [retained control](../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/decoded-results.json) | `4dc32dddeb5a60b758a03b04d0a9271e83bdf37c92662d9c7e7b4e519f1d89a9` |

## Targeted new N11/N13 materialisations

[Persistence run](../../../f5/answers/nh20261012persist1/ANSWERS.md) retains RULE_INIT seed, native-procedure/event reads, exact namespace/regular-global/static cells, CLIENT_ACCEPTED connection nonces and bounded paired keep-alive requests. It also measures a computed ordinary head against a native provider, exact `info commands`/`info procs` catalogue versus native call, and computed rename of only the owned native route with before/after/restore receipts. These are actual native definitions, independent of legacy dynamic Tcl-procedure examples. The completed [independent persistence review](../../../f5/answers/nh20261012persist1/independent-review.json) checks every source/hash/raw line, actual invoking event, paired peer ports and cell readout. The [reviewed summary](../../../f5/answers/nh20261012persist1/README.md) retains 287 measured records and 95 narrow selected proofs. No all-worker traffic or full numbered-question completion is asserted here. The run measured four initial units and actual HTTP unit `0:0`; ordered cleanup receipts inspect exact owned cells/procedure slots on each initial unit.

Legacy proc-semantics logs truncate values to sixty characters and concern dynamically created Tcl procs. They do not supply exact-byte native-catalog visibility/rename evidence. Legacy CLIENT_ACCEPTED examples seed the same constant on every connection, which supplies no per-connection isolation proof. The reviewed paired requests retain A port 51992 and B port 51993 on actual unit `0:0`: A retains its connection write while B keeps its separate seed; both observe shared-cell WRITE, missing-after-unset, and RECREATE through event/native-procedure reads. A later fresh port 51994 has a new connection seed and the same worker RECREATE values. These exact peer/value sequences settle that bounded isolation and persistence distinction.

The five matrix questions keep `answer_record` null while their wider variants are unsettled. `attempt_record` links an actual bounded native record or an explicitly named capability boundary. The retained source mappings under `materialised_input` identify concrete provider/caller configurations without inventing an exact portable source.
