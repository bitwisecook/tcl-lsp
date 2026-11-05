# naming.variable.original-frame-array-trace-caller-access

Kind: `native-observation`

## Problem statement

An unset callback during a procedure return can use upvar1 while a caller has a same-spelled array. Complete before-return values and callback logs are needed to distinguish those public caller accesses from mutation of the retiring owner's old members.

## Question

What whole before-return/caller values and callback logs do the two original frame-teardown sources produce when unset callbacks use upvar1 and add traces through the resulting caller aliases?

## Conclusion

Each pinned C8.4–C9.1 provider completes both originals with code0. The root-access source returns {OWNER PARENT {{unset parentNew}}} {{ROOT PARENT} OLD PARENTNEW}; the member-access source returns {{OWNERX OWNERY} PARENTX PARENTY} {{OLD PARENTX PARENTY} {OLD PARENTX PARENTY} PARENTNEW PARENTNEW PARENTNEW PARENTNEW}. In these whole finite programs, the original owner's before-return reads retain OWNER or OWNERX/OWNERY, the callback upvar1 reads report the caller's PARENT or PARENTX/PARENTY values, and newly added caller traces have their exact later public log entries. These values do not establish accessible old-owner mutation, private frame/root/member/registration identity, arbitrary callback order or a universal teardown algorithm. Jim rejects trace setup with guest code1 and invalid command name "trace" for both sources; it supplies no frame unset-callback answer. All twelve standalone driver processes exit0 with empty stderr and retain separate VERSION and ORIGINAL counted-hex channels. No Native frame/argv/header grant or Rust assertion pass follows from matching public values.

## Scope

Two untouched ASCII LF sources each execute independently in six fresh fully initialised interpreters under the unchanged counted-source CAPI driver, without an added original-source envelope. Twelve whole process streams/receipts and six compile receipts/ELFs retain original source hashes, executable/archive/header/build/required-source pins and exact complete results. The initial request and expanded capture request remain distinct inputs. The owner-teardown347 bad-level inputs and frame-teardown348 namespace-alias restrictions are independent limited controls, with no substituted source/result or donated frame chronology. Opaque/NUL/Unicode names, private identities, BIG-IP and executed software controls are not tested. Current linked comparisons consume complete unchanged public code/result windows without host refusal, keeping admitted namespace/caller observations separate from rejected frame setups and no executed Rust pass claim. Software namespace teardown retains its actual original parent edge while callbacks run and retires only the selected root binding; same-spelled foreign state and private receiver correspondence require their independent software owners. Public logs do not prove those private identities.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Two independent fresh fullInit original counted-source evaluations; VERSION and ORIGINAL hex completion/result rows remain separate, without an added source envelope. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Both originals measure guest code0 with exact results frame-root-access='{OWNER PARENT {{unset parentNew}}} {{ROOT PARENT} OLD PARENTNEW}'; frame-member-access='{{OWNERX OWNERY} PARENTX PARENTY} {{OLD PARENTX PARENTY} {OLD PARENTX PARENTY} PARENTNEW PARENTNEW PARENTNEW PARENTNEW}'. Whole before-return/caller reads and callback logs are measured; private frame, retiring member and registration identity are unobserved. Both independent host processes exit0 with empty stderr.

### jim

Status: `observed`. Channel: Two independent fresh fullInit original counted-source evaluations; VERSION and ORIGINAL hex completion/result rows remain separate, without an added source envelope. Version: jim 0.84-9-g5bac7c9.

Both whole originals measure guest code1 and invalid command name "trace"; no frame unset callback/caller-access answer is supplied. Both independent host processes exit0 with empty stderr.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/interp/native_variable_teardown.rs](../../../../runtime/rust/src/interp/native_variable_teardown.rs), `interp::native_variable_teardown::tests::original_frame_callbacks_observe_caller_without_donating_member_traces` (linked): All twelve untouched349 full public before-return/caller values and callback logs compare, including Jim trace refusal; no private old-frame mutation/registration identity or runtime algorithm is inferred.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_array_frame_access349/8.4.20](../../../../rust/tcl-registry/tests/data/native_array_frame_access349/8.4.20)
- [rust/tcl-registry/tests/data/native_array_frame_access349/8.5.19](../../../../rust/tcl-registry/tests/data/native_array_frame_access349/8.5.19)
- [rust/tcl-registry/tests/data/native_array_frame_access349/8.6.18](../../../../rust/tcl-registry/tests/data/native_array_frame_access349/8.6.18)
- [rust/tcl-registry/tests/data/native_array_frame_access349/9.0.4](../../../../rust/tcl-registry/tests/data/native_array_frame_access349/9.0.4)
- [rust/tcl-registry/tests/data/native_array_frame_access349/9.1.0](../../../../rust/tcl-registry/tests/data/native_array_frame_access349/9.1.0)
- [rust/tcl-registry/tests](../../../../rust/tcl-registry/tests)
- [rust/tcl-registry/tests/data/native_array_frame_access349/jim](../../../../rust/tcl-registry/tests/data/native_array_frame_access349/jim)
