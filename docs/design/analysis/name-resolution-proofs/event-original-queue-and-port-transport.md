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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`. Channel: Not executed by this record. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

Separate selected pure recipe/port implementation contract; actual native answers are in the finite Event observation/source records.

## Consumer bindings

- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `EventQueue`: Generic original-handle retention and turn topology.
- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs), `NativeEventProtocol`: Independently selected C/Jim event recipes.
- [rust/tcl-vm/src/interp/native_event_context.rs](../../../../rust/tcl-vm/src/interp/native_event_context.rs), `Vm::with_event_global_frame`: Actual VM global-frame selection and restoration.
- [runtime/rust/src/interp/native_event_context.rs](../../../../runtime/rust/src/interp/native_event_context.rs), `Interp::with_event_global_frame`: Actual Runtime global-frame selection and restoration.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::bgerror_apply`: Retain the original valid prefix and release the replaced owning handle after the prefix RefCell borrow ends; no callback-free, ordinary-release or Normal grant.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::process_bg_errors`: Move pending original message/options objects out of the queue borrow before dispatch; preserve original prefix members and caller result.
- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `event::tests::current_event_turn_preserves_original_scripts_and_live_cancellation` (linked): Original Rc byte handle identity, cancellation before the next current entry, and deferred newly registered generation. No Rust execution claim.

## Evidence

- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs)
- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs)
- [rust/tcl-vm/src/cmd_event.rs](../../../../rust/tcl-vm/src/cmd_event.rs)
- [runtime/rust/src/cmd_event.rs](../../../../runtime/rust/src/cmd_event.rs)
- [rust/tcl-vm/src/interp/native_event_context.rs](../../../../rust/tcl-vm/src/interp/native_event_context.rs)
- [runtime/rust/src/interp/native_event_context.rs](../../../../runtime/rust/src/interp/native_event_context.rs)
