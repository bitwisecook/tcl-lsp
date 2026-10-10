# naming.event.original-queue-and-port-transport

Kind: `implementation-contract`

## Problem statement

Rendered event scripts and detached ready batches lose original byte objects, live cancellation and independently selected wait/evaluation purposes.

## Question

Does the shared queue retain original script handles and live cancellation while both ports consume selected callback/global wait purposes?

## Conclusion

The shared generic EventQueue owns unchanged script handles, services only current-turn generations and skips cancelled live entries. Registry selects the actual C/Jim recipe. VM Value and Runtime Owned adapters retain genuine objects, original prefixes and actual global frames. This is an implementation contract, not a native object lifecycle or ordinary completion proof.

## Scope

Basic after/vwait/update integration. Native validation lives in the separate finite Event questions. Missing host notifier completeness remains a refusal; no native CPP, original physical cache, callback-free or normal completion permission is granted. No Rust execution result is recorded. The Runtime prefix replacement and background queue extraction end table borrows before old object release or callback dispatch. This is Rust ownership choreography; no custom free-callback native observation or successful completion is inferred.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: 8.4.20. Build: Authored Rust implementation; no backend execution inferred.. Channel: Not executed by this record.. Dialect: tcl8.4.

Separate selected pure recipe/port implementation contract; actual native answers are in the finite Event observation/source records.

### tcl8.5

Status: `not-tested`. Version: 8.5.19. Build: Authored Rust implementation; no backend execution inferred.. Channel: Not executed by this record.. Dialect: tcl8.5.

Separate selected pure recipe/port implementation contract; actual native answers are in the finite Event observation/source records.

### tcl8.6

Status: `not-tested`. Version: 8.6.18. Build: Authored Rust implementation; no backend execution inferred.. Channel: Not executed by this record.. Dialect: tcl8.6.

Separate selected pure recipe/port implementation contract; actual native answers are in the finite Event observation/source records.

### tcl9.0

Status: `not-tested`. Version: 9.0.4. Build: Authored Rust implementation; no backend execution inferred.. Channel: Not executed by this record.. Dialect: tcl9.0.

Separate selected pure recipe/port implementation contract; actual native answers are in the finite Event observation/source records.

### tcl9.1

Status: `not-tested`. Version: 9.1.0. Build: Authored Rust implementation; no backend execution inferred.. Channel: Not executed by this record.. Dialect: tcl9.1.

Separate selected pure recipe/port implementation contract; actual native answers are in the finite Event observation/source records.

### jim

Status: `not-tested`. Version: 0.84-9-g5bac7c9. Build: Authored Rust implementation; no backend execution inferred.. Channel: Not executed by this record.. Dialect: jim.

Separate selected pure recipe/port implementation contract; actual native answers are in the finite Event observation/source records.

### bigip

Status: `not-tested`. Version: not tested. Build: Authored Rust implementation; no backend execution inferred.. Channel: Not executed by this record.. Dialect: bigip.

Separate selected pure recipe/port implementation contract; actual native answers are in the finite Event observation/source records.

## Exact evidence

- `implementation0` (implementation): [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs). SHA-256 `9817803a7b9def02c773794477100ed15b4285416f47c7d1f029a408f208c63c`. Current implementation source only; no native launch or Rust execution inferred.
- `implementation1` (implementation): [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs). SHA-256 `0f75a6a5eb87d4605974cecdd0d99be4e012ba5e34e0d75af3f2216b95046550`. Current implementation source only; no native launch or Rust execution inferred.
- `implementation2` (implementation): [rust/tcl-vm/src/cmd_event.rs](../../../../rust/tcl-vm/src/cmd_event.rs). SHA-256 `a8f669a4bdfb26cd37443ca60c6f22b31e374c2e67334150ec6d2e1b4b0e1955`. Current implementation source only; no native launch or Rust execution inferred.
- `implementation3` (implementation): [runtime/rust/src/cmd_event.rs](../../../../runtime/rust/src/cmd_event.rs). SHA-256 `e0e05beefbe1273c0c6c9148381ed1ec6533b8e5d633b27f3fec1cc5231466c7`. Current implementation source only; no native launch or Rust execution inferred.
- `implementation4` (implementation): [rust/tcl-vm/src/interp/native_event_context.rs](../../../../rust/tcl-vm/src/interp/native_event_context.rs). SHA-256 `36610cf3e2527a16177e372f48d2c86d6ee51a1162c9fe791bdf466c2427e96e`. Current implementation source only; no native launch or Rust execution inferred.
- `implementation5` (implementation): [runtime/rust/src/interp/native_event_context.rs](../../../../runtime/rust/src/interp/native_event_context.rs). SHA-256 `d36529b6605058a42a15a45ea75415e762ec50de5af1a72e238b65f82704fea2`. Current implementation source only; no native launch or Rust execution inferred.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `EventQueue`: Generic original-handle retention and turn topology.
- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs), `NativeEventProtocol`: Independently selected C/Jim event recipes.
- [rust/tcl-vm/src/interp/native_event_context.rs](../../../../rust/tcl-vm/src/interp/native_event_context.rs), `Vm::with_event_global_frame`: Actual VM global-frame selection and restoration.
- [runtime/rust/src/interp/native_event_context.rs](../../../../runtime/rust/src/interp/native_event_context.rs), `Interp::with_event_global_frame`: Actual Runtime global-frame selection and restoration.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::bgerror_apply`: Retain the original valid prefix and release the replaced owning handle after the prefix RefCell borrow ends; no callback-free, ordinary-release or Normal grant.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::process_bg_errors`: Move pending original message/options objects out of the queue borrow before dispatch; preserve original prefix members and caller result.
- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `event::tests::current_event_turn_preserves_original_scripts_and_live_cancellation` (linked): Original Rc byte handle identity, cancellation before the next current entry, and deferred newly registered generation. No Rust execution claim.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked Rust tests state original queue/object ownership assertions. Independent exact-source receipts record execution outcomes; native callback and transport observations retain their separate questions.
