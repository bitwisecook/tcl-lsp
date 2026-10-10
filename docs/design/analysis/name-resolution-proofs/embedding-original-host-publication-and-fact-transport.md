# Original host publication and fact transport

Proof ID: `naming.embedding.original-host-publication-and-fact-transport`. Kind: `implementation-contract`.

## Problem statement

Embedding publication keys, callback values and scalar facts can outlive their original interpreter, namespace or binding, or cross a guest completion boundary. A counted name, profile, payload or reporting key alone cannot authorize a current publication, reconstruct an original object view or turn a host-only refusal into guest return options.

## Question

How does the Runtime embedding interface authenticate original command-publication ownership and transport exact callback/completion/scalar facts while retaining scope, generation, descriptor origin, storage and host-refusal boundaries?

## Conclusion

Owned publication receipts retain the actual Rust interpreter, original counted name and key, namespace incarnation, selected naming protocol, purpose and occupancy generation. Immediate creation/deletion authenticate these complete facts once; stale, counterfeit, foreign, consumed, retired-scope or expired-interpreter claims refuse. Callback registrars retain their original scope, and whitelists retain actual host/unit tokens through rename. Unclassified guest retirement work refuses before effects; host reentry-created replacement generations survive removal of the old registration. Materialized callback strings preserve exact counted bytes. Supported scalar snapshots and resident storage retain independently reported full payload/origin/tags, while unavailable original-object views, foreign descriptor origins and unknown adoption storage refuse. Pure scalar codecs transport retained facts only. Guest completion/result/options use the existing original completion owner; typed host refusal remains outside guest catch, preserves prior effects and retained interpreter completion state, and precedes guest capture.

## Scope

Ten fixed software controls bind eight Runtime embedding ownership/transport contracts and two neutral scalar codecs. Authored C-release fixtures select software protocols and storage constructors; they do not run or observe an external Tcl provider. The record contains no executed assertion claim. Private Rust namespace/binding generations are independent of C private table identity. Supported snapshot transport supplies no repeated original object header, C-produced getter/cache, native callback equivalence, deferred overlay admission, compiler instruction, physical frame or reached native Handler. Existing Native ABI experiments retain their separate measured scopes; all C/Jim/BIG-IP provider answers for this implementation question remain not tested.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This implementation question has no external provider execution receipt. Fixed Rust embedding and carrier inputs establish software ownership/transport obligations, not native table, callback, object/cache/header, Handler, frame or completion behaviour.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This implementation question has no external provider execution receipt. Fixed Rust embedding and carrier inputs establish software ownership/transport obligations, not native table, callback, object/cache/header, Handler, frame or completion behaviour.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This implementation question has no external provider execution receipt. Fixed Rust embedding and carrier inputs establish software ownership/transport obligations, not native table, callback, object/cache/header, Handler, frame or completion behaviour.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This implementation question has no external provider execution receipt. Fixed Rust embedding and carrier inputs establish software ownership/transport obligations, not native table, callback, object/cache/header, Handler, frame or completion behaviour.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This implementation question has no external provider execution receipt. Fixed Rust embedding and carrier inputs establish software ownership/transport obligations, not native table, callback, object/cache/header, Handler, frame or completion behaviour.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This implementation question has no external provider execution receipt. Fixed Rust embedding and carrier inputs establish software ownership/transport obligations, not native table, callback, object/cache/header, Handler, frame or completion behaviour.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This implementation question has no external provider execution receipt. Fixed Rust embedding and carrier inputs establish software ownership/transport obligations, not native table, callback, object/cache/header, Handler, frame or completion behaviour.

## Exact evidence

- `naming-embedding-original-host-publication-and-fact-transport-engine-definition` (implementation): [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs). SHA-256 `3c7f88f8a3ac2203f33b685c6302f7d6226997849e5307712ca4dd9027dabf00`. Current Root-ACKed405/406 software interface and eight marked assertion definitions; no external provider or executed assertion claim.
- `naming-embedding-original-host-publication-and-fact-transport-carrier-definition` (implementation): [rust/tcl-syntax/src/scalar_getter/carrier.rs](../../../../rust/tcl-syntax/src/scalar_getter/carrier.rs). SHA-256 `2a43e8b7597ab2c2c58f018658d115b0d1b990ac166a5358f45ac9aa99665d27`. Lossless scalar/storage transport definitions with two canonical comment markers; all non-comment bytes unchanged. No native cache production is asserted.

## Source inspection

The current Rust interface and codec definitions supply the named software contracts. No upstream Tcl source excerpt or external experiment observes these Rust receipt/generation identities. The independent [Native command publication and callback scope](native-abi-command-publication-callback-and-scope.md), [original object callback getters](native-abi-original-object-callback-getters.md) and [callback result/options](native-abi-callback-completion-result-and-options.md) records retain their measured inputs and authority boundaries. The [child host refusal contract](interpreter-original-child-host-refusal-transport.md) separately binds typed cause transport and prior-parent result precedence.

## Consumer bindings

- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `RuntimeEngine::command_publication_service`: Request an owned immediate publication service from the actual interpreter; positive selected Native naming policy and a live scope are required. Logical compatibility and pending deferred overlay are independent and cannot donate this authority.
- [runtime/rust/src/interp/native_host_publication.rs](../../../../runtime/rust/src/interp/native_host_publication.rs), `PublicationService::open`: Retain a weak actual interpreter, original namespace incarnation, selected name protocol and actual host registration owner for callback-held publication.
- [runtime/rust/src/interp/native_host_publication.rs](../../../../runtime/rust/src/interp/native_host_publication.rs), `PublicationService::prepare`: Select the actual Rust creation/deletion slot under the recorded purpose, retain original counted bytes and occupancy generation, and classify required guest retirement effects before issuing a one-use receipt.
- [runtime/rust/src/interp/native_host_publication.rs](../../../../runtime/rust/src/interp/native_host_publication.rs), `authenticate`: Require the same actual interpreter, complete original key/name, unconsumed purpose, live policy/scope and exact current occupancy. Counterfeit, stale or foreign receipt data is terminal refusal.
- [runtime/rust/src/interp/native_host_publication.rs](../../../../runtime/rust/src/interp/native_host_publication.rs), `consume`: Authenticate the requested publication purpose, recheck classified retirement and consume the one-use receipt before returning its actual retained slot. No source label or profile reconstructs authority.
- [runtime/rust/src/interp/native_host_publication.rs](../../../../runtime/rust/src/interp/native_host_publication.rs), `PublicationService::classify_retirement`: Refuse unhandled synchronous guest traces, coroutine handoffs and unknown deletion callbacks before host publication/removal; the source contract does not assert native lifecycle equivalence.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `RuntimeEngine::restrict_commands`: Preserve actual registered host and compiled-unit tokens through the shared whitelist operation, independently of renamed or opaque reporting spellings.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `host_command_proc`: Retain actual callback registrar scope and declared argument view. Counted strings and supported snapshots have separate conversions; unavailable original-object bridge refuses before materializing an alternative callback view.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `remove_prepared_host_command`: Keep the selected original generation across host retirement callbacks and leave callback-created replacements intact after exact current-generation comparison.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `to_obj`: Adopt only supported complete retained scalar facts after independent current descriptor-origin/dialect and resident-storage validation. The codec alone supplies no receiving object permission.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `snapshot_value`: Transport the runtime object snapshot and its recorded full cache/storage through neutral facts without claiming repeated external header identity or a C-produced getter observation.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `fail`: Keep guest script errors/options on the shared completion path while typed host access/refusal remains outside guest catch and retains prior interpreter completion/result state.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `capture_answer`: Check retained host refusal before guest result/options capture; exact result bytes and explicit guest completion use their independent transport owner.
- [rust/tcl-syntax/src/scalar_getter/carrier.rs](../../../../rust/tcl-syntax/src/scalar_getter/carrier.rs), `import_scalar`: Transport the full retained scalar payload and independent word-Boolean release without guessing the receiver profile or granting cache adoption.
- [rust/tcl-syntax/src/scalar_getter/carrier.rs](../../../../rust/tcl-syntax/src/scalar_getter/carrier.rs), `export_scalar`: Retain full scalar magnitude, radix and IEEE/NaN information; a word Boolean requires its separately recorded descriptor origin and cannot borrow a default release.
- [rust/tcl-syntax/src/scalar_getter/carrier.rs](../../../../rust/tcl-syntax/src/scalar_getter/carrier.rs), `import_storage`: Preserve the recorded canonical-empty, allocated or unknown storage classification independently of byte extent or live object authority.
- [rust/tcl-syntax/src/scalar_getter/carrier.rs](../../../../rust/tcl-syntax/src/scalar_getter/carrier.rs), `export_storage`: Transport the explicit recorded storage tag without manufacturing allocation identity or inferring storage from an empty string.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `engine::tests::publication_receipts_keep_actual_slots_and_refuse_forged_stale_or_foreign_authority` (linked): Owned publication receipts authenticate the actual Rust interpreter, original counted name/key, namespace incarnation, purpose and occupancy generation. Forged, consumed, renamed/rebound, foreign, retired-scope and expired-interpreter receipts refuse; a genuine absent deletion is distinct from a stale claim.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `engine::tests::callback_scope_is_owned_and_c_creation_keeps_unqualified_global_rule` (linked): A callback-retained publication service owns its actual original scope. The selected C-style unqualified creation rule targets the Rust global namespace, a qualified target retains its own namespace and a pending overlay refuses service publication; no C table observation follows.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `engine::tests::whitelist_keeps_renamed_opaque_host_token_and_exact_unit_tokens` (linked): Whitelisting retains actual registered host and compiled-unit generations through rename, including opaque counted names. A stale old spelling supplies no command; symbolic profile or public inventory names do not substitute for those Rust tokens.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `engine::tests::callbacks_and_results_keep_exact_counted_bytes_and_scalar_storage` (linked): Host materialized-string callbacks preserve counted NUL/non-UTF8 bytes. Explicitly retained supported scalar/cache/resident-storage snapshots stay distinct from string values; foreign word-Boolean origin and unknown storage refuse adoption. These authored facts do not claim C-produced caches or repeated header identity.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `engine::tests::host_refusal_is_uncatchable_and_retains_prior_effects_and_completion_state` (linked): Typed host execution refusal remains outside guest catch/completion, retains earlier effects and prevents later effects. The model keeps the prior result/resident bytes, pending return state/options and prior-parent host cause when transporting a child refusal.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `engine::tests::guest_failure_options_and_nonstandard_completions_use_shared_original_owner` (linked): Guest ScriptBytes errors and explicit nonstandard completion/options use the shared original return-options owner and retain counted result/options fields, independently of typed host refusal. This is Rust embedding transport, not a native worker result.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `engine::tests::retirement_callback_reentry_protects_new_generation_and_guest_traces_refuse_before_effects` (linked): Host retirement reentry can publish a new actual generation that survives the old removal. Required synchronous guest deletion traces refuse before effects rather than being silently omitted; private Rust generations are not measured C identities.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `engine::tests::original_object_view_and_logical_native_receipts_refuse_explicitly` (linked): Logical engines refuse native publication services while explicit compatibility registration remains separate. An unavailable original-object callback bridge refuses before callback/later guest effects instead of manufacturing a native object view.
- [rust/tcl-syntax/src/scalar_getter/carrier.rs](../../../../rust/tcl-syntax/src/scalar_getter/carrier.rs), `scalar_getter::carrier::tests::scalar_carriers_preserve_full_payload_and_independent_origins` (linked): Pure carrier round trips retain full integer/bignum sign/radix/magnitude, NaN payload/sign, double bits and independently recorded word-Boolean release. Narrow getter results and dialect labels do not replace the original retained payload.
- [rust/tcl-syntax/src/scalar_getter/carrier.rs](../../../../rust/tcl-syntax/src/scalar_getter/carrier.rs), `scalar_getter::carrier::tests::missing_boolean_origin_refuses_and_storage_is_never_inferred` (linked): A word Boolean without its recorded descriptor origin refuses export. Canonical-empty, allocated and unknown storage tags round trip independently; byte length and conversion do not manufacture allocation or native-object authority.

A coverage binding records a defined assertion, not an executed result.

## Replay

Named tests bind current authored software assertions without claiming that they ran. Genuine test artifacts and closed verification receipts record their independent outcomes. Native ABI publication/getter/completion experiments remain separately scoped and are not replayed or promoted here.
