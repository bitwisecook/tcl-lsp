# Active VM artifact manifest revalidation

Proof ID: `naming.compiler.active-manifest-revalidation`. Kind: `implementation-contract`.

## Problem statement

A compiled software activation can retain a manifest and pack-dependent site claims while the held pack facts, package floors or overlay context differ. A reporting profile or changed command epoch alone does not revalidate the original artifact identity.

## Question

How do VM compiled entry and active command-boundary revalidation use the actual retained manifest and current coherent pack claims while keeping Generic dispatch and independent Native admission purposes separate?

## Conclusion

VM compiled entry and active command-epoch revalidation join actual pack site claims with the retained artifact manifest through function_identity_claims_match. Each site claim requires its exact current pack stamp and coherence. Manifest disagreements map to dependent rungs through ArtefactIdentityManifest::refused_rungs; a Generic assembly without those dependencies remains independent. Existing compiled-local, compiler, command and procedure guards remain required. A missing manifest does not supply missing pack facts. Same-profile package-floor changes can refuse dependent activation without changing profile generation. At a stale source command boundary, unavailable exact-source replay keeps the admitted epoch and program counter rather than acknowledging the changed pack state. Refused overlay pinning retains the current context and artifact identity. These are software gate definitions, not Native behavior or test execution results.

## Scope

Four deterministic VM software controls define entry pack/manifest admission, active changed-pack and same-profile package-floor refusal, Generic independence, stale-boundary replay refusal without a compile service, and atomic OverlayMiss refusal. Authentic C8.6 core is used only for object/result storage. All C Tcl, Jim and BIG-IP providers are not tested for this question. No Native process, original header/table identity, compiler instruction/body equivalence, physical frame/namespace roster, successful handler or external completion is established. Manifest/rung keys do not replace independently current activation, command/procedure and physical storage prerequisites. No authored control is claimed executed.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No external provider execution answers this software implementation contract. The authentic C8.6 VM fixture supplies object/result storage only; it is not evidence of Tcl provider, handler, compiler, frame or command-table behavior.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No external provider execution answers this software implementation contract. The authentic C8.6 VM fixture supplies object/result storage only; it is not evidence of Tcl provider, handler, compiler, frame or command-table behavior.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No external provider execution answers this software implementation contract. The authentic C8.6 VM fixture supplies object/result storage only; it is not evidence of Tcl provider, handler, compiler, frame or command-table behavior.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No external provider execution answers this software implementation contract. The authentic C8.6 VM fixture supplies object/result storage only; it is not evidence of Tcl provider, handler, compiler, frame or command-table behavior.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No external provider execution answers this software implementation contract. The authentic C8.6 VM fixture supplies object/result storage only; it is not evidence of Tcl provider, handler, compiler, frame or command-table behavior.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No external provider execution answers this software implementation contract. The authentic C8.6 VM fixture supplies object/result storage only; it is not evidence of Tcl provider, handler, compiler, frame or command-table behavior.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: f5-bigip.

No external provider execution answers this software implementation contract. The authentic C8.6 VM fixture supplies object/result storage only; it is not evidence of Tcl provider, handler, compiler, frame or command-table behavior.

## Exact evidence

- `naming-compiler-active-manifest-revalidation-rust-tcl-runtime-api-src-manifest.rs` (implementation): [rust/tcl-runtime-api/src/manifest.rs](../../../../rust/tcl-runtime-api/src/manifest.rs). SHA-256 `8d657a2abac9ee2e7adfe57992b76f2977ff12fe8b2188ecf469c12afe5ed235`. Current shared software gate or authored control definition; no executed Rust assertion or external Native provider result.
- `naming-compiler-active-manifest-revalidation-rust-tcl-vm-src-interp.rs` (implementation): [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs). SHA-256 `ef1a7598ff00f01347317df786eb20f9654da8af98bedf9241584764be8f80eb`. Current shared software gate or authored control definition; no executed Rust assertion or external Native provider result.
- `naming-compiler-active-manifest-revalidation-rust-tcl-vm-src-exec.rs` (implementation): [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs). SHA-256 `508d40cacc19a5cc3ec3dcb5df952791574c116a116017d0823ab3cc66af0338`. Current shared software gate or authored control definition; no executed Rust assertion or external Native provider result.
- `naming-compiler-active-manifest-revalidation-rust-tcl-vm-src-exec-active_manifest_tests.rs` (implementation): [rust/tcl-vm/src/exec/active_manifest_tests.rs](../../../../rust/tcl-vm/src/exec/active_manifest_tests.rs). SHA-256 `d773a3b1192a965ec0237dcbca6a0ef24db1a8bf81c22f2cfcdc52e114e388b4`. Current shared software gate or authored control definition; no executed Rust assertion or external Native provider result.

## Source inspection

The listed Rust sources define the current software gate and its control inputs. No upstream Tcl implementation excerpt or external Native experiment is attached to this question.

## Consumer bindings

- [rust/tcl-runtime-api/src/manifest.rs](../../../../rust/tcl-runtime-api/src/manifest.rs), `ArtefactIdentityManifest::refused_rungs`: Map actual retained-versus-held manifest disagreements to their dependent software rungs; agreement creates no compiler, handler, table roster or physical activation authority.
- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `Vm::run_compiled_unit`: Require existing compiled-local and native compiler prerequisites plus shared identity/pack claims before constructing the software activation.
- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `Frame::manifest`: Carry the actual retained artifact manifest into command-epoch revalidation, independently of the reported profile and original Native frame/layout proofs.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::function_identity_claims_match`: Join current coherent site claims with the retained manifest/rung check at entry and live revalidation; neither check issues command/procedure/compiler or activation authority.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::function_live_command_bindings_match`: Keep existing compiled-local, selected command and procedure guards while validating actual site claims against the active frame manifest.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::manifest_admits`: Refuse only assembly rungs depending on changed actual manifest fields; Generic dispatch remains independent when it carries no dependent claims. A missing manifest cannot create an unavailable pack stamp.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::site_claims_hold`: Require each exact pack content/vocabulary/overlay/evaluator stamp and claim coherence from the actual VM; reference-body claims additionally retain their procedure binding and Tcl-body backing.
- [rust/tcl-vm/src/exec/active_manifest_tests.rs](../../../../rust/tcl-vm/src/exec/active_manifest_tests.rs), `exec::active_manifest_tests::compiled_entry_checks_retained_manifest_and_actual_pack_claims` (linked): Software entry control defines admission with the actual retained manifest and current pack stamp; removed pack facts and missing-manifest assembly carrying unavailable pack claims refuse. The authentic C8.6 VM fixture provides object/result storage only, not an external provider completion proof.
- [rust/tcl-vm/src/exec/active_manifest_tests.rs](../../../../rust/tcl-vm/src/exec/active_manifest_tests.rs), `exec::active_manifest_tests::active_currency_rechecks_pack_claims_and_manifest_package_floors` (linked): Software active-frame control withdraws a pack-dependent assembly after pack removal and after same-profile package-floor changes, using the retained frame manifest. A generic assembly without dependent claims remains independent. This asserts no Native compiler, handler or namespace/frame roster currency.
- [rust/tcl-vm/src/exec/active_manifest_tests.rs](../../../../rust/tcl-vm/src/exec/active_manifest_tests.rs), `exec::active_manifest_tests::stale_pack_command_boundary_does_not_acknowledge_the_changed_epoch` (linked): Software command-boundary control retains the stale activation epoch and program counter when exact-source replay lacks a compile service; changed pack facts cannot be acknowledged by executing the stale pack-specialised instruction. No external Tcl instruction or completion equivalence is asserted.
- [rust/tcl-vm/src/exec/active_manifest_tests.rs](../../../../rust/tcl-vm/src/exec/active_manifest_tests.rs), `exec::active_manifest_tests::refused_overlay_pin_preserves_the_active_artifact_identity` (linked): Software pinning control keeps the actual context, held artifact identity and command epoch unchanged after an OverlayMiss refusal; retained active claims continue to match that unchanged software state. It establishes no Native overlay installation, handler or physical frame identity.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named software controls are linked to current source bodies, not executed-result claims. Library compilation and no-run test-build receipts retain independent scopes; they supply no external provider outcome or proof of these assertions.
