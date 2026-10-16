# Logical alias destination applicability repair

Patch: logical-alias-applicability.patch; SHA256 and exact two current-main beforeimages/proposed images are in manifest.json. Both files were copied from current main after mapped47767 integration. No Cargo/main/Git writes.

The failing original test remains `namespace eval N {global $unknown}` with expected unresolved alias-name obligations `[]`. Its inputs and assertion are unchanged. The genuine existing Logical source scope supports a NamespaceActivation, but the adapter asked `VariableAliasDestination::is_active_in_frame` using the unversioned Logical invocation dialect. That ordinary API correctly retains unknown: plain Tcl has no execution variable_lookup_policy, and ProcedureLocal in Namespace depends on that policy. Root Global no-op succeeds because that branch is policy-independent.

The repair adds `ResolvedInvocation::authored_logical_source_alias_destination_is_active_in_frame`. It requires a positively selected unversioned Tcl source schema, no Native point/version/name protocol/execution lookup policy, valid actual argv cardinality, and membership of the requested destination in that schema's actual authored Logical aliases. It asks the existing destination classifier under each of the five C Tcl release policy records and accepts only unanimity. Unknown source frame, missing/other descriptor, dynamic/unavailable source alias grammar, non-Tcl/Native point or contradictory execution policy refuses. No command-name resolver or duplicate alias grammar was added. This supplies conditional source applicability only; it cannot grant a Native frame/link, completed effect or ordinary absence.

The Compiler adopter still validates its actual SourceTransitions words, complete input, original source image and realm, and obtains the original frame from the existing independent Logical scope. Missing frame remains Unknown and keeps obligations. Only a genuine inactive selected source destination removes its source naming obligation. Alias continuity, cells, original ordinary transitions, Native policy and coverage proofs remain separate.

The exact old failing test now also asserts its actual retained NamespaceActivation and that the ordinary destination API still returns None with plain Logical policy. Thus another real frame-transport failure will fail explicitly, not be hidden by a fallback. A new Registry test checks Global/Namespace source no-op, Procedure source active, Unknown refusal, nonmember descriptor refusal, Native dialect facade refusal, unchanged original state transitions and unchanged ordinary unknown execution policy.

Source provenance: source-provenance.json pins the five original native Tcl_GlobalObjCmd entries and binaries. All five actual executables produce namespace:NAMESPACE and procedure:ROOT for the fixed original probe. The first diagnostic probe used `set` inside a namespace and encountered Tcl's root-variable fallback; its source and stdout were preserved separately, then the diagnostic was corrected to genuinely declare the namespace variable using `variable`. This does not change any Compiler fixture or claim an earlier successful capture. Original entry guards return before name getters outside procedure/local frames.

Rust tests were not run by author. Isolated rustfmt parsed both files and patch dry-run is clean. Root acceptance commands:

```
cargo test --locked -p tcl-registry --lib logical_alias_destination_applicability_
cargo test --locked -p tcl-compiler --lib logical_alias_name_obligations_
```

The six logical_variable_ predecessor controls already passed in root's frozen2d582 run, independently of this new candidate. No new Rust PASS or completion count is claimed.
