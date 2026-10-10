# naming.invocation.original-variable-inventory-policy

Kind: `implementation-contract`

## Problem statement

Variable inventory needs the actual selected naming purpose before pattern conversion or table access. Catalogue compatibility and a missing lookup policy cannot establish that purpose or the implementation family that owns the inventory.

## Question

How do shared info vars/globals consumers retain the actual sealed C/Jim variable-inventory policy and refuse unsupported or missing policy before a pattern getter or guest-state effect?

## Conclusion

NativeNameProtocol::variable_lookup_policy projects Tcl or Jim from its already selected recipe. NamePolicyProtocol forwards that projection while retaining authority independently. Runtime and VM Namespaces::variable_lookup_policy delegate their actual name_policy_protocol rather than deriving a replacement recipe from a catalogue profile. Shared info::vars and info::globals require selected_variable_inventory_policy before pattern conversion, inventory collection or result publication. An unsupported actual naming purpose returns the typed variable inventory lookup policy refusal. The selected-purpose controls cover six supported software backend selections; the unsupported Jim0.79 catalogue projection supplies no selected recipe, leaves the nonresident original pattern untouched and preserves prior Runtime guest result.

## Scope

Three marked software/API definitions cover actual selected policy plus simple variable results, VM refusal before pattern getter, and Runtime refusal with prior guest result retained. These are software admission/storage/result controls, not original external C/Jim process comparisons or Jim0.79 observations. Catalogue release/compatibility, physical table/frame currency, actual object getter authority and provider authority remain independent. A pure sealed policy projection confers no Native entry, reached handler, private variable identity, completed source evaluation, object/cache observation or complete inventory law. All seven external providers remain not tested; no executed Rust assertion is attached by this binding.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No original external provider process answers this selected variable-inventory policy contract. Linked controls describe software policy/admission and preservation of original pattern/result state; Native table/frame/handler behaviour remains independent.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No original external provider process answers this selected variable-inventory policy contract. Linked controls describe software policy/admission and preservation of original pattern/result state; Native table/frame/handler behaviour remains independent.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No original external provider process answers this selected variable-inventory policy contract. Linked controls describe software policy/admission and preservation of original pattern/result state; Native table/frame/handler behaviour remains independent.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No original external provider process answers this selected variable-inventory policy contract. Linked controls describe software policy/admission and preservation of original pattern/result state; Native table/frame/handler behaviour remains independent.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No original external provider process answers this selected variable-inventory policy contract. Linked controls describe software policy/admission and preservation of original pattern/result state; Native table/frame/handler behaviour remains independent.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No original external provider process answers this selected variable-inventory policy contract. Linked controls describe software policy/admission and preservation of original pattern/result state; Native table/frame/handler behaviour remains independent.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No original external provider process answers this selected variable-inventory policy contract. Linked controls describe software policy/admission and preservation of original pattern/result state; Native table/frame/handler behaviour remains independent.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::variable_lookup_policy`: Project variable lookup semantics only from the already selected C or Jim recipe; catalogue compatibility cannot create a recipe.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NamePolicyProtocol::variable_lookup_policy`: Forward sealed naming-purpose policy while retaining independent provider/source authority.
- [runtime/rust/src/state_traits.rs](../../../../runtime/rust/src/state_traits.rs), `Namespaces::variable_lookup_policy`: Delegate the actual Runtime selected name_policy_protocol and preserve unsupported purpose as None.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Namespaces::variable_lookup_policy`: Delegate the actual VM selected name_policy_protocol; no nominal catalogue fallback supplies missing inventory policy.
- [rust/tcl-cmd-core/src/info.rs](../../../../rust/tcl-cmd-core/src/info.rs), `selected_variable_inventory_policy`: Require a selected policy before any pattern getter/table scan; missing purpose becomes typed refusal.
- [rust/tcl-cmd-core/src/info.rs](../../../../rust/tcl-cmd-core/src/info.rs), `vars`: Use independently admitted policy before variable collection and actual pattern access.
- [rust/tcl-cmd-core/src/info.rs](../../../../rust/tcl-cmd-core/src/info.rs), `globals`: Use independently admitted policy before global collection and actual pattern access.
- [rust/tcl-vm/src/interp/native_variable_inventory_policy_tests.rs](../../../../rust/tcl-vm/src/interp/native_variable_inventory_policy_tests.rs), `interp::native_variable_inventory_policy_tests::variable_inventory_uses_the_actual_selected_name_purpose` (linked): Six supported software backend selections delegate the actual sealed name purpose and compare simple retained variable results. This is not an original external provider outcome.
- [rust/tcl-vm/src/interp/native_variable_inventory_policy_tests.rs](../../../../rust/tcl-vm/src/interp/native_variable_inventory_policy_tests.rs), `interp::native_variable_inventory_policy_tests::unsupported_inventory_policy_refuses_before_a_pattern_getter` (linked): Unsupported Jim0.79 catalogue projection has no actual name policy; both shared inventory queries refuse before a nonresident original pattern getter. No Jim0.79 process or Native cache observation is claimed.
- [runtime/rust/src/state_traits.rs](../../../../runtime/rust/src/state_traits.rs), `state_traits::tests::unsupported_variable_inventory_policy_preserves_original_guest_state` (linked): Unsupported actual inventory purpose refuses both queries without giving the original wide pattern a string representation or changing the prior Runtime guest result. Software ownership/state control only.

A named test is a coverage binding, not a claim that it executed.

## Replay

Current software definitions are linked only. Assertion coverage requires the independently compiled/listed/executed image matching those definitions. No original process, Native/private table/frame or mutable implementation digest is supplied.
