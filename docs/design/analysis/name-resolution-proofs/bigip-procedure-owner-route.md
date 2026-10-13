# naming.bigip.procedure-owner-route

Kind: `native-observation`

## Problem statement

An F5 rule::proc call resembles a Tcl qualified command. Joining it through ordinary Tcl namespace lookup, a folder-tail guess or configuration-object lifetime would select the wrong procedure or incorrectly remove a live route.

## Question

How do owner-local and absolute partition/folder F5 call routes behave before activation, after provider configuration deletion and across event frames?

## Conclusion

Owner-local call selects the calling rule; partition-root relative routing and absolute folder/partition paths have different measured doors. Tcl-rooted call names and namespace which are counterexamples. Activated calls survive the recorded provider-config deletion. The event-local CLIENT_SEED remains readable in the five reached later events; no suspended-frame replacement is proved.

## Scope

The original canonical and expandedr2286i programs, four responding TMMs, recorded owner/folder/partition paths and six events; future versions and entered suspended frames are excluded. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported.

Owner-local call selects the calling rule; partition-root relative routing and absolute folder/partition paths have different measured doors. Tcl-rooted call names and namespace which are counterexamples. Activated calls survive the recorded provider-config deletion. The event-local CLIENT_SEED remains readable in the five reached later events; no suspended-frame replacement is proved.

The measured report rows and their limits are:


The original owner-local controls and the expanded `r2286i` folder/partition matrix reached units 0, 1, 2 and 3. Every 128-request traffic batch produced exactly 32 responses from each actual unit. The three literal-emoji traffic batches produced 16 from each unit.

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

The original procedure results are in [canonical raw LTM log](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/ltm-raw.log`). Expanded proof is in the [r2286i raw LTM log](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/ltm-raw.log`), [attachment transcripts](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/attachments`), [traffic responses](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/dev/traffic`), and [backend observations](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/dev/backend.log`).


## Exact evidence

[bigip-procedure-owner-route.json](bigip-procedure-owner-route.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.

```json
[]
```
