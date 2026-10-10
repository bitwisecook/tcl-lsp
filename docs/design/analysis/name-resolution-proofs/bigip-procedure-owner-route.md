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

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: The original canonical and expandedr2286i programs, four responding TMMs, recorded owner/folder/partition paths and six events; future versions and entered suspended frames are excluded.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

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

The original procedure results are in [canonical raw LTM log](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/ltm-raw.log). Expanded proof is in the [r2286i raw LTM log](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/ltm-raw.log), [attachment transcripts](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/attachments), [traffic responses](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/dev/traffic), and [backend observations](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/dev/backend.log).


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md). SHA-256 `02a0c0e69ba8ed1ceee716fdca5be0c40bc0592bde086213c557b52c6d637136`. Lines 84–110. Exact build-specific measured report section; all rows and limits remain in the retained report.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md). SHA-256 `02a0c0e69ba8ed1ceee716fdca5be0c40bc0592bde086213c557b52c6d637136`. Lines 3–3. Actual appliance version attribution for this report; no separately named hotfix inferred.
- `raw-0` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/ltm-raw.log](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/ltm-raw.log). SHA-256 `d1b804b6eb1587c7cbff4e71f267cac9e8f3a4d1a63feb3d41943cda9a80c48b`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `raw-1` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/ltm-raw.log](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/ltm-raw.log). SHA-256 `cdf50547d7d4e025c778c295b95c293719d2e9699c1a28f44edb474d096ff691`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `raw-2` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/dev/backend.log](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/dev/backend.log). SHA-256 `e28b2c5eff77e37d0813240ae2c1b2ad4a401cf195cd6895137744ad050a83f1`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `source-closure-5` (provider): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/inventory.txt](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/inventory.txt). SHA-256 `99cc929a22a600c11a700fff0b2dc1cb7333ed525bcdb87ab041c9a1187e45ec`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `source-closure-6` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/manifest-verification.txt](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/appliance/manifest-verification.txt). SHA-256 `f800077eaadc0104201639706c8d912d3804118dd6175be3f9cd727052d1bbbe`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `source-closure-7` (input): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/manifest.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/manifest.json). SHA-256 `8f5c9f385b226b5f7da6885ede6a28775a68cdfea7ed44fb28fb213c73f6e6fd`. Retained report-specific source manifest or appliance inventory; individual measured inputs remain distinct.
- `aggregate-8` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/result.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/result.json). SHA-256 `ff299fa606867041acd7975db9d13cca40e5ba7b6135f8228d22168d5388d725`. Inspected measured canonical case aggregate and its explicit unmeasured limits; actual case rows remain in this exact file.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
