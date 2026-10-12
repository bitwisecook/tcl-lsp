# First actual OO target/facet refusal

The actual current dependency build passed (exit 0); the unchanged exact C9.1 control executed one test and failed (exit 101). No argument, Value factory, fixture, original caller or assertion changed.

First failing original target predicate:

```text
OO_TARGET owner=1 interpreter=0 target=1 stage=current.namespace_owner current=false
OO_NAMESPACE token=25 arena_present=true world_live=true declared=false intern_same=true dying=false deferred_owner=false retained=false
```

The original receiver object is present; its cache allocation remains live/identical, creation and namespace tokens match, and destroyed state matches. Its actual namespace owner then fails: arena path and live world exist, intern still names the same token, but the declaration set is absent and no dying/deferred/retained owner supplies the other accepted route. This is the measured first gate; publishing a namespace merely to satisfy it is not a repair.

The [full log](control.log) retains the same complete neutral first cause, `ValueAccessRefusal(CommandProtocolUnavailable("original OO Name target/facet"))`, during class construction. The intended callback/restoration assertions remain unreached. This supplies no semantic pass or conclusion about all other failed controls.

[execution-record.json](execution-record.json), [Cargo receipt](build.jsonl), [build stderr](build.stderr.txt), [exact name check](control-list.log), [harness](run.py) and [plan](plan.json) retain the exact commands, identities, scope and exits. Binary SHA256 `04cbacef6d85d51eda1b94fef0fd83af4b4d9fa2074136671b643dc1286e539a` stayed unchanged during execution. It is explicitly non-current for restored source; no post-restoration rebuild ran.

The [before](source-before.json), [instrumented](source-instrumented.json) and [restored](source-restored.json) snapshots cover 4943 Rust source/manifest/toolchain files, including the actual uncommitted clock/callback/shared changes compiled for this probe; no published-HEAD claim is made. This deliberately broad scope includes unlinked runtime files and is not a compiled-dependency closure or whole-input identity claim. Runtime-only edits were held until restoration, so all recorded broad hashes matched after restoration. Only the two declared temporary VM owners differed during instrumentation; both are byte-exact restored. Original before bytes are retained separately.

The issues-cache lease and runtime-only edit hold were released immediately after restoration. No source fix, assertion weakening, Cargo repeat, canonical proof/Git mutation, network or raw F5 read occurred. Next work must establish the genuine bootstrap/private-namespace creation/lifecycle route from actual source and matching C9.1 evidence.
