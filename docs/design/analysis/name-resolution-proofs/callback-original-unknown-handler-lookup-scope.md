# naming.callback.original-unknown-handler-lookup-scope

Kind: `native-observation`

## Problem statement

A relative namespace/package unknown callback may resolve in the installer, trigger or global namespace. Equal callback text does not select its dispatch scope, and unsupported APIs must remain measured failures rather than invented compatibility.

## Question

Which publicly observed shadow callback runs for the original relative namespace unknown and package unknown controls, and which exact APIs are unavailable on each pinned provider?

## Conclusion

Six exact standalone original CLI processes and six independent version-query processes exit zero with empty stderr. C8.5.19/C8.6.18/C9.0.4/C9.1.0 namespace unknown installation succeeds and the original namespace trigger returns code0/handled with observed TRIGGER, selecting the authored trigger shadow in this finite control. C8.4.20 namespace unknown installation fails with the full bad-option result, and its missing-command trigger returns code1 with observed empty. Jim 0.84-9-g5bac7c9 namespace unknown installation fails with its full unsupported-subcommand result, and trigger returns code1 with observed empty. All five C package unknown installers return code0/empty; package require returns code1/can't find package resolution_scope_missing_package with observed GLOBAL, so callback observation remains distinct from unsuccessful package completion. Jim package unknown installation is unsupported and require returns code1/Can't load package resolution_scope_missing_package with observed empty. Whole original five-row stdout per provider, independent actual versions and exact error spellings are authoritative. These public markers establish relative callback routing among the actual original shadows only; they supply no private entered-frame/cell identity, future callback lookup, object/cache/header, TMM result, installation lifetime or general arbitrary callback law. BIG-IP is not tested.

## Scope

One exact ASCII LF standalone source contains original catches, three explicitly authored namespace shadows and two separate namespace/package callback controls. Each provider runs the unchanged original file directly; independent version.tcl is a separate process. No evaluation/catch/puts wrapper is added by the launcher. Executable/static-library/header/build/source pins are retained and rechecked. Registry lookup-purpose descriptors and conditional source registration remain independently linked implementation contracts; current provider observations do not execute their Rust assertions.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Exact standalone ASCII LF CLI original script, explicit catch as written; version separately executed, no surrounding evaluation wrapper; observed callback markers are public results, no private-frame/TMM claim. Version: tcl8.4 8.4.20.

Namespace unknown is unavailable: original installation returns code1 with its full bad-option table, creation code0/empty and missing-command trigger code1/invalid command with observed empty. Package installation succeeds code0/empty; require returns code1/can't find package resolution_scope_missing_package with observed GLOBAL. The exact original and independent version CLI processes each exit0 with empty stderr; these finite authored shadows supply no private frame/current cell or future lookup identity.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact standalone ASCII LF CLI original script, explicit catch as written; version separately executed, no surrounding evaluation wrapper; observed callback markers are public results, no private-frame/TMM claim. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Namespace installation returns code0/resolution_scope_handler, creation code0/empty and trigger code0/handled with observed TRIGGER. Package installation returns code0/empty; require returns code1/can't find package resolution_scope_missing_package with observed GLOBAL. Trigger routing is publicly observed while package completion remains failed. The exact original and independent version CLI processes each exit0 with empty stderr; these finite authored shadows supply no private frame/current cell or future lookup identity.

### jim

Status: `observed`. Channel: Exact standalone ASCII LF CLI original script, explicit catch as written; version separately executed, no surrounding evaluation wrapper; observed callback markers are public results, no private-frame/TMM claim. Version: jim 0.84-9-g5bac7c9.

Namespace unknown installation returns code1 with its exact unsupported subcommand table; namespace creation returns code0/resolution_scope_handler and missing-command trigger code1 with observed empty. Package unknown installation returns code1 with its exact unavailable subcommand table; require returns code1/Can't load package resolution_scope_missing_package with observed empty. No callback is observed and alternative API is not substituted. The exact original and independent version CLI processes each exit0 with empty stderr; these finite authored shadows supply no private frame/current cell or future lookup identity.

### bigip

Status: `not-tested`.

No appliance execution answers these exact original standalone callback-scope controls.

## Consumer bindings

- [rust/tcl-registry/src/selected_script_timing.rs](../../../../rust/tcl-registry/src/selected_script_timing.rs), `SelectedScriptMetadata::lookup_arguments`: Project independently selected Registry source callback lookup purpose; native public observations remain separate.
- [rust/tcl-registry/src/selected_script_timing.rs](../../../../rust/tcl-registry/src/selected_script_timing.rs), `selected_script_timing::tests::original_unknown_handler_scopes_require_selected_supported_callback_operands` (linked): Actual selected availability projects C85+ namespace TriggerFrame and all5C package GlobalFrame descriptors, unavailable C84/Jim controls and literal/dynamic/query/expanded/non-callback refusal. This is metadata selection, not observed callback entry or physical frame.

## Evidence

- [rust/tcl-registry/tests/data/native_callback_scope240/8.4.20](../../../../rust/tcl-registry/tests/data/native_callback_scope240/8.4.20)
- [rust/tcl-registry/tests/data/native_callback_scope240/8.5.19](../../../../rust/tcl-registry/tests/data/native_callback_scope240/8.5.19)
- [rust/tcl-registry/tests/data/native_callback_scope240/8.6.18](../../../../rust/tcl-registry/tests/data/native_callback_scope240/8.6.18)
- [rust/tcl-registry/tests/data/native_callback_scope240/9.0.4](../../../../rust/tcl-registry/tests/data/native_callback_scope240/9.0.4)
- [rust/tcl-registry/tests/data/native_callback_scope240/9.1.0](../../../../rust/tcl-registry/tests/data/native_callback_scope240/9.1.0)
- [rust/tcl-registry/tests/data/native_callback_scope240/jim](../../../../rust/tcl-registry/tests/data/native_callback_scope240/jim)
- [rust/tcl-registry/tests/data/native_callback_scope240](../../../../rust/tcl-registry/tests/data/native_callback_scope240)
